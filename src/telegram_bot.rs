/*
 * Personal and Educational Free License (PEF License / PEFL)
 * Version 1.0 - 2026-05-26
 * Copyright (c) 2026 Rubens Rainelli
 *
 * Contact Details:
 *   Email: rubens@rainelli.it
 *   GitHub: https://github.com/RubensRainelli/
 *   Tel: (+39) 342 811 5882
 *
 * Personal and educational free use permitted.
 * Corporate, business or enterprise use prohibited without written authorization and fee.
 * The full license can be found in the LICENSE file.
 */

use crate::harnesses::{
    available_models_for_harness, dispatch_to_harness, scan_local_conversations,
    PromptDispatchSpec, TaskCompletionSummary,
};
use crate::i18n::{resolve_locale, t};
use crate::json::Json;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const HARNESS_LIST: &[&str] = &["antigravity", "claude-code", "opencode", "deepseek", "aider"];
pub const THINKING_LEVELS: &[&str] = &["none", "low", "medium", "high", "max"];
pub const PERMISSION_MODES: &[&str] = &["plan", "ask", "auto_edit", "auto"];

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BotConfig {
    pub mode: String, // "cloud" | "direct"
    pub bot_name: String,
    pub bot_token: String,
    pub cloud_url: String,
    pub device_token: String,
    pub webapp_url: String,
    pub default_locale: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ConversationSession {
    pub conversation_id: String,
    pub project_name: String,
    pub workspace: String,
    pub title: String,
    pub owner_user_id: i64,
    pub chat_id: i64,
    pub topic_id: Option<i64>,
    pub harness: String,
    pub model: String,
    pub thinking_level: String,
    pub permission_mode: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CollaboratorEntry {
    pub project_name: String,
    pub user_id: i64,
    pub username: String,
    pub role: String, // "owner" | "executor" | "proposer" | "viewer"
    pub max_permission_mode: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct PendingOwnerPrompt {
    pub id: String,
    pub spec: PromptDispatchSpec,
    pub project_name: String,
    pub topic_title: String,
    pub author_id: i64,
    pub author_name: String,
    pub origin_chat_id: i64,
    pub origin_topic_id: Option<i64>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InteractiveQuestionState {
    pub id: String,
    pub conversation_id: String,
    pub project_name: String,
    pub kind: String, // "single_select" | "multi_select" | "tool_approval"
    pub tool_name: String,
    pub question_text: String,
    pub options: Vec<String>,
    pub selected_indices: Vec<usize>,
    pub resolved: bool,
    pub allow: bool,
    pub chosen_labels: Vec<String>,
    pub custom_reply: Option<String>,
}

pub struct BotRuntimeState {
    pub owner_user_id: Option<i64>,
    pub active_pairing_code: Option<String>,
    pub user_locales: HashMap<i64, String>,
    pub sessions: HashMap<String, ConversationSession>,
    pub topic_to_conv: HashMap<(i64, i64), String>,
    pub collaborators: Vec<CollaboratorEntry>,
    pub invite_tokens: HashMap<String, (String, String)>, // token -> (project_name, role)
    pub group_default_roles: HashMap<i64, String>,        // group_chat_id -> "viewer" | "proposer" | "executor"
    pub pending_owner_prompts: HashMap<String, PendingOwnerPrompt>,
    pub questions: HashMap<String, InteractiveQuestionState>,
}

pub type SharedBotState = Arc<(Mutex<BotRuntimeState>, Condvar)>;

pub fn load_bot_config() -> BotConfig {
    let mut env_map: HashMap<String, String> = HashMap::new();
    if let Ok(content) = fs::read_to_string(".env") {
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                env_map.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
            }
        }
    }

    let get_val = |key: &str, default: &str| -> String {
        env::var(key)
            .ok()
            .or_else(|| env_map.get(key).cloned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string())
    };

    let bot_token = get_val("TELEGRAM_BOT_TOKEN", "");
    let raw_mode = get_val("THM_MODE", "");
    let mode = if !raw_mode.is_empty() {
        raw_mode
    } else if !bot_token.is_empty() && bot_token != "YOUR_TELEGRAM_BOT_TOKEN_HERE" {
        "direct".to_string()
    } else {
        "cloud".to_string()
    };

    // Load or generate personal device_token (stored in `.thm/device.json`)
    let device_token = get_or_create_device_token(&get_val("THM_DEVICE_TOKEN", ""));

    BotConfig {
        mode,
        bot_name: get_val("TELEGRAM_BOT_NAME", "@Your_Harness_Bot"),
        bot_token,
        cloud_url: get_val("THM_CLOUD_URL", "https://app.tgcloud.ai"),
        device_token,
        webapp_url: get_val(
            "THM_WEBAPP_URL",
            "https://rubensrainelli.github.io/telegram-harness-manager/",
        ),
        default_locale: get_val("THM_DEFAULT_LOCALE", "en"),
    }
}

pub fn get_or_create_device_token(env_token: &str) -> String {
    if !env_token.is_empty() && env_token != "YOUR_PERSONAL_DEVICE_TOKEN_FROM_PAIRING" {
        return env_token.to_string();
    }
    let dir = Path::new(".thm");
    let _ = fs::create_dir_all(dir);
    let file = dir.join("device.json");
    if let Ok(raw) = fs::read_to_string(&file) {
        if let Some(j) = Json::parse_opt(&raw) {
            if let Some(tok) = j.get_str("deviceToken") {
                return tok.to_string();
            }
        }
    }
    let generated = format!("thm_dev_{}_{}", now_secs(), (now_millis() % 999_999));
    let info = Json::obj(vec![
        ("deviceId", Json::Str(format!("pc_{}", now_secs() % 100_000))),
        ("deviceToken", Json::Str(generated.clone())),
        ("createdAt", Json::Number(now_secs() as f64)),
    ]);
    let _ = fs::write(&file, info.stringify());
    generated
}

pub fn new_shared_bot_state() -> SharedBotState {
    let mut owner_id = None;
    let mut pairing_code = None;
    if let Ok(raw) = fs::read_to_string(".thm/device.json") {
        if let Some(j) = Json::parse_opt(&raw) {
            owner_id = j.get_i64("ownerUserId");
            pairing_code = j.get_str("pairingCode").map(|s| s.to_string());
        }
    }

    Arc::new((
        Mutex::new(BotRuntimeState {
            owner_user_id: owner_id,
            active_pairing_code: pairing_code,
            user_locales: HashMap::new(),
            sessions: HashMap::new(),
            topic_to_conv: HashMap::new(),
            collaborators: Vec::new(),
            invite_tokens: HashMap::new(),
            group_default_roles: HashMap::new(),
            pending_owner_prompts: HashMap::new(),
            questions: HashMap::new(),
        }),
        Condvar::new(),
    ))
}

/// Generates a one-time cryptographic pairing code and deep-link for `@Your_Harness_Bot`
pub fn generate_pairing_code(cfg: &BotConfig, state: &SharedBotState) -> (String, String) {
    let code = format!("{:06x}{:04x}", now_secs() & 0xFFFFFF, (now_millis() & 0xFFFF) as u32);
    let existing_owner = {
        let mut guard = state.0.lock().unwrap();
        guard.active_pairing_code = Some(code.clone());
        guard.owner_user_id
    };

    let _ = fs::create_dir_all(".thm");
    let mut entries = vec![
        ("deviceToken", Json::Str(cfg.device_token.clone())),
        ("pairingCode", Json::Str(code.clone())),
        ("createdAt", Json::Number(now_secs() as f64)),
    ];
    if let Some(oid) = existing_owner {
        entries.push(("ownerUserId", Json::Number(oid as f64)));
    }
    let record = Json::obj(entries);
    let _ = fs::write(".thm/device.json", record.stringify());

    // If in Cloud mode, also register the pairing code on the Telegram Serverless endpoint
    if cfg.mode == "cloud" && !cfg.cloud_url.contains("YOUR_TGCLOUD_APP_ID") {
        let payload = Json::obj(vec![
            ("action", Json::Str("register_pair".into())),
            ("code", Json::Str(code.clone())),
            ("deviceId", Json::Str("local-pc".into())),
            ("deviceName", Json::Str(get_hostname())),
            ("deviceTokenHash", Json::Str(cfg.device_token.clone())),
            ("osName", Json::Str(env::consts::OS.into())),
        ]);
        let _ = https_post_json(&format!("{}/api/bridge", cfg.cloud_url), &payload, None);
    }

    let clean_bot = cfg.bot_name.trim_start_matches('@');
    let deep_link = format!("https://t.me/{}?start=pair_{}", clean_bot, code);
    (code, deep_link)
}

/// Starts the background Telegram Bot / Serverless Bridge worker thread
pub fn spawn_telegram_worker(cfg: BotConfig, state: SharedBotState) {
    thread::spawn(move || {
        if cfg.mode == "direct"
            && !cfg.bot_token.is_empty()
            && cfg.bot_token != "YOUR_TELEGRAM_BOT_TOKEN_HERE"
        {
            println!(
                "🤖 [Telegram Direct Bot Mode] Connected as {} (Multi-Topic & ACL Active)",
                cfg.bot_name
            );
            // Automatically discover and sync local projects & conversations into Telegram Forum Topics
            let _ = sync_local_projects_and_topics(&cfg, &state);
            run_direct_telegram_polling_loop(cfg, state);
        } else {
            println!(
                "☁️ [Telegram Cloud Bridge Mode] Polling Serverless Bridge for {} (Zero Bot Token Exposure)",
                cfg.bot_name
            );
            run_cloud_bridge_polling_loop(cfg, state);
        }
    });
}

/// Scans real local conversations (`~/.gemini/antigravity/brain`) and ensures each one has
/// its own dedicated Telegram Forum Topic (`createForumTopic`) with its Config Bar and latest status.
pub fn sync_local_projects_and_topics(cfg: &BotConfig, state: &SharedBotState) -> usize {
    if cfg.mode != "direct"
        || cfg.bot_token.is_empty()
        || cfg.bot_token == "YOUR_TELEGRAM_BOT_TOKEN_HERE"
    {
        return 0;
    }

    let (owner_id, lang) = {
        let guard = state.0.lock().unwrap();
        let oid = match guard.owner_user_id {
            Some(id) if id != 0 => id,
            _ => return 0,
        };
        let loc = guard
            .user_locales
            .get(&oid)
            .cloned()
            .unwrap_or_else(|| cfg.default_locale.clone());
        (oid, loc)
    };

    let _ = fs::create_dir_all(".thm");
    let map_path = Path::new(".thm").join("topics_map.json");
    let mut persisted_map: BTreeMap<String, i64> = BTreeMap::new();
    if let Ok(raw) = fs::read_to_string(&map_path) {
        if let Some(Json::Object(obj)) = Json::parse_opt(&raw) {
            for (k, v) in obj {
                if let Some(tid) = v.as_i64() {
                    persisted_map.insert(k, tid);
                }
            }
        }
    }

    // Clean up old test topic 1563967 if present and unmapped
    if !persisted_map.values().any(|&v| v == 1563967) {
        let _ = telegram_api_call(
            &cfg.bot_token,
            "deleteForumTopic",
            &Json::obj(vec![
                ("chat_id", Json::Number(owner_id as f64)),
                ("message_thread_id", Json::Number(1563967.0)),
            ]),
        );
    }

    let discovered = scan_local_conversations(8);
    let mut synced_count = 0usize;

    for disc in discovered {
        let topic_title = format!(
            "📁 [{}] 💬 {}",
            disc.project_name,
            truncate_str(&disc.title, 42)
        );

        let mut thread_id = persisted_map.get(&disc.conversation_id).copied();
        let mut is_new_topic = false;

        if thread_id.is_none() {
            if let Ok(res) = telegram_api_call(
                &cfg.bot_token,
                "createForumTopic",
                &Json::obj(vec![
                    ("chat_id", Json::Number(owner_id as f64)),
                    ("name", Json::Str(topic_title.clone())),
                ]),
            ) {
                if let Some(tid) = res.get("result").and_then(|r| r.get_i64("message_thread_id")) {
                    thread_id = Some(tid);
                    persisted_map.insert(disc.conversation_id.clone(), tid);
                    is_new_topic = true;
                }
            }
        }

        let sess = ConversationSession {
            conversation_id: disc.conversation_id.clone(),
            project_name: disc.project_name.clone(),
            workspace: disc.workspace.clone(),
            title: topic_title.clone(),
            owner_user_id: owner_id,
            chat_id: owner_id,
            topic_id: thread_id,
            harness: disc.harness.clone(),
            model: "gemini-4-argon".to_string(),
            thinking_level: "medium".to_string(),
            permission_mode: "ask".to_string(),
        };

        {
            let mut guard = state.0.lock().unwrap();
            if let Some(tid) = thread_id {
                guard
                    .topic_to_conv
                    .insert((owner_id, tid), disc.conversation_id.clone());
            }
            guard
                .sessions
                .insert(disc.conversation_id.clone(), sess.clone());
        }

        if is_new_topic {
            send_or_edit_topic_config_bar(cfg, owner_id, thread_id, None, &sess, &lang);
            let status_html = format!(
                "🗂 <b>Progetto:</b> <code>{}</code>\n🔑 <b>Session ID:</b> <code>{}</code>\n📊 <b>Passaggi registrati:</b> {}\n\n<b>Ultimo Prompt:</b>\n<blockquote>{}</blockquote>\n\n<b>Ultimo Output:</b>\n{}\n\n<blockquote expandable><b>🧠 Ultimo Thinking &amp; Contesto</b>\n{}</blockquote>",
                escape_html(&disc.project_name),
                escape_html(&disc.conversation_id[..disc.conversation_id.len().min(8)]),
                disc.step_count,
                escape_html(if disc.last_user_prompt.is_empty() { "Sessione attiva" } else { &disc.last_user_prompt }),
                escape_html(if disc.last_output.is_empty() { "In attesa di istruzioni." } else { &disc.last_output }),
                escape_html(if disc.last_thinking.is_empty() { "Nessun blocco thinking aggiuntivo." } else { &disc.last_thinking })
            );
            let _ = send_telegram_html(&cfg.bot_token, owner_id, thread_id, &status_html, None);
        }

        synced_count += 1;
    }

    // Persist updated topic mappings
    let mut map_entries = Vec::new();
    for (k, v) in &persisted_map {
        map_entries.push((k.as_str(), Json::Number(*v as f64)));
    }
    let _ = fs::write(&map_path, Json::obj(map_entries).stringify());

    synced_count
}

fn run_cloud_bridge_polling_loop(cfg: BotConfig, state: SharedBotState) {
    loop {
        thread::sleep(Duration::from_secs(3));
        if cfg.cloud_url.contains("YOUR_TGCLOUD_APP_ID") {
            continue;
        }
        let endpoint = format!("{}/api/bridge", cfg.cloud_url.trim_end_matches('/'));
        let poll_body = Json::obj(vec![("action", Json::Str("poll".into()))]);
        if let Ok(resp_str) = https_post_json(&endpoint, &poll_body, Some(&cfg.device_token)) {
            if let Some(resp) = Json::parse_opt(&resp_str) {
                if let Some(prompts) = resp.get_arr("prompts") {
                    for p in prompts {
                        let spec = PromptDispatchSpec {
                            conversation_id: p.get_str("conversation_id").unwrap_or("new").to_string(),
                            workspace: env::current_dir()
                                .map(|d| d.display().to_string())
                                .unwrap_or_default(),
                            harness: p.get_str("harness").unwrap_or("antigravity").to_string(),
                            model: p.get_str("model").unwrap_or("gemini-4-argon").to_string(),
                            thinking_level: p.get_str("thinking_level").unwrap_or("medium").to_string(),
                            permission_mode: p.get_str("permission_mode").unwrap_or("ask").to_string(),
                            prompt: p.get_str("prompt_text").unwrap_or("").to_string(),
                        };
                        let _ = dispatch_to_harness(&spec);
                    }
                }
                if let Some(resolved) = resp.get_arr("resolvedQuestions") {
                    if !resolved.is_empty() {
                        let (lock, cvar) = &*state;
                        if let Ok(mut guard) = lock.lock() {
                            for rq in resolved {
                                if let Some(qid) = rq.get_str("id") {
                                    if let Some(q_item) = guard.questions.get_mut(qid) {
                                        q_item.resolved = true;
                                        q_item.allow = true;
                                        cvar.notify_all();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn run_direct_telegram_polling_loop(cfg: BotConfig, state: SharedBotState) {
    let mut offset: i64 = 0;

    loop {
        let get_updates_req = Json::obj(vec![
            ("offset", Json::Number(offset as f64)),
            ("timeout", Json::Number(15.0)),
            (
                "allowed_updates",
                Json::Array(vec![
                    Json::Str("message".into()),
                    Json::Str("callback_query".into()),
                    Json::Str("my_chat_member".into()),
                ]),
            ),
        ]);

        match telegram_api_call(&cfg.bot_token, "getUpdates", &get_updates_req) {
            Ok(resp) => {
                if let Some(updates) = resp.get_arr("result") {
                    for upd in updates {
                        if let Some(uid) = upd.get_i64("update_id") {
                            if uid >= offset {
                                offset = uid + 1;
                            }
                        }
                        process_telegram_update(&cfg, &state, upd);
                    }
                }
            }
            Err(_) => {
                thread::sleep(Duration::from_secs(3));
            }
        }
    }
}

fn process_telegram_update(cfg: &BotConfig, state: &SharedBotState, upd: &Json) {
    if let Some(member_upd) = upd.get("my_chat_member") {
        handle_group_membership_update(cfg, state, member_upd);
        return;
    }

    if let Some(cb) = upd.get("callback_query") {
        handle_callback_query(cfg, state, cb);
        return;
    }

    if let Some(msg) = upd.get("message") {
        handle_incoming_message(cfg, state, msg);
    }
}

/// Group Forum Security & Pre-flight Check:
/// 1. Only the verified Owner of the Harness can add the bot to a group.
/// 2. Checks that Forum Topics (`is_forum`) are enabled and the bot has `can_manage_topics` permission.
/// 3. Locks default group member role to `viewer` (Read-Only) so nobody can trigger accidental prompts.
fn handle_group_membership_update(cfg: &BotConfig, state: &SharedBotState, member_upd: &Json) {
    let chat = match member_upd.get("chat") {
        Some(c) => c,
        None => return,
    };
    let chat_type = chat.get_str("type").unwrap_or("");
    if chat_type != "supergroup" && chat_type != "group" {
        return;
    }
    let chat_id = chat.get_i64("id").unwrap_or(0);
    let from = match member_upd.get("from") {
        Some(f) => f,
        None => return,
    };
    let from_id = from.get_i64("id").unwrap_or(0);
    let lang = resolve_locale(from.get_str("language_code"));

    let new_status = member_upd
        .get("new_chat_member")
        .and_then(|m| m.get_str("status"))
        .unwrap_or("");
    if new_status == "left" || new_status == "kicked" {
        return;
    }

    let is_owner = {
        let guard = state.0.lock().unwrap();
        guard.owner_user_id == Some(from_id)
    };

    if !is_owner {
        let _ = send_telegram_html(
            &cfg.bot_token,
            chat_id,
            None,
            &t(Some(&lang), "bot.group_not_owner", &[]),
            None,
        );
        let _ = telegram_api_call(
            &cfg.bot_token,
            "leaveChat",
            &Json::obj(vec![("chat_id", Json::Number(chat_id as f64))]),
        );
        return;
    }

    verify_and_configure_group(cfg, state, chat_id, &lang);
}

fn verify_and_configure_group(cfg: &BotConfig, state: &SharedBotState, chat_id: i64, lang: &str) {
    let chat_res = telegram_api_call(
        &cfg.bot_token,
        "getChat",
        &Json::obj(vec![("chat_id", Json::Number(chat_id as f64))]),
    )
    .ok();
    let is_forum = chat_res
        .as_ref()
        .and_then(|r| r.get("result"))
        .and_then(|c| c.get_bool("is_forum"))
        .unwrap_or(false);

    let me_res = telegram_api_call(&cfg.bot_token, "getMe", &Json::obj(vec![])).ok();
    let bot_id = me_res
        .as_ref()
        .and_then(|r| r.get("result"))
        .and_then(|m| m.get_i64("id"))
        .unwrap_or(0);

    let member_res = telegram_api_call(
        &cfg.bot_token,
        "getChatMember",
        &Json::obj(vec![
            ("chat_id", Json::Number(chat_id as f64)),
            ("user_id", Json::Number(bot_id as f64)),
        ]),
    )
    .ok();

    let bot_member = member_res.as_ref().and_then(|r| r.get("result"));
    let is_admin = bot_member
        .and_then(|m| m.get_str("status"))
        .map(|s| s == "administrator")
        .unwrap_or(false);
    let can_manage_topics = bot_member
        .and_then(|m| m.get_bool("can_manage_topics"))
        .unwrap_or(false);

    // Default role for group members is always locked to "viewer" (Read-Only) initially
    if let Ok(mut guard) = state.0.lock() {
        guard
            .group_default_roles
            .entry(chat_id)
            .or_insert_with(|| "viewer".to_string());
    }

    if !is_forum || !is_admin || !can_manage_topics {
        let msg = t(
            Some(lang),
            "bot.group_check_failed",
            &[
                ("topics_status", if is_forum { "✅ OK" } else { "❌ OFF" }),
                ("admin_status", if is_admin { "✅ OK" } else { "❌ OFF" }),
                (
                    "manage_topics_status",
                    if can_manage_topics { "✅ OK" } else { "❌ OFF" },
                ),
            ],
        );
        let kb = Json::obj(vec![(
            "inline_keyboard",
            Json::Array(vec![Json::Array(vec![Json::obj(vec![
                (
                    "text",
                    Json::Str(t(Some(lang), "buttons.recheck_group", &[])),
                ),
                (
                    "callback_data",
                    Json::Str(format!("grp_recheck:{}", chat_id)),
                ),
            ])])]),
        )]);
        let _ = send_telegram_html(&cfg.bot_token, chat_id, None, &msg, Some(kb));
        return;
    }

    let ok_msg = t(Some(lang), "bot.group_setup_ok", &[]);
    let kb = Json::obj(vec![(
        "inline_keyboard",
        Json::Array(vec![
            Json::Array(vec![
                Json::obj(vec![
                    ("text", Json::Str("👁️ Default: Viewer (Read-Only)".into())),
                    (
                        "callback_data",
                        Json::Str(format!("grp_role:{}:viewer", chat_id)),
                    ),
                ]),
                Json::obj(vec![
                    ("text", Json::Str("✋ Default: Proposer".into())),
                    (
                        "callback_data",
                        Json::Str(format!("grp_role:{}:proposer", chat_id)),
                    ),
                ]),
            ]),
            Json::Array(vec![Json::obj(vec![
                (
                    "text",
                    Json::Str(t(Some(lang), "buttons.new_conversation", &[])),
                ),
                ("callback_data", Json::Str("new_topic".into())),
            ])]),
        ]),
    )]);
    let _ = send_telegram_html(&cfg.bot_token, chat_id, None, &ok_msg, Some(kb));
}

fn handle_incoming_message(cfg: &BotConfig, state: &SharedBotState, msg: &Json) {
    let from = match msg.get("from") {
        Some(f) => f,
        None => return,
    };
    if from.get_bool("is_bot").unwrap_or(false) {
        return;
    }
    let user_id = from.get_i64("id").unwrap_or(0);
    let username = from
        .get_str("username")
        .map(|u| format!("@{}", u))
        .or_else(|| from.get_str("first_name").map(|s| s.to_string()))
        .unwrap_or_else(|| format!("User {}", user_id));
    let lang = resolve_locale(from.get_str("language_code"));

    let chat = match msg.get("chat") {
        Some(c) => c,
        None => return,
    };
    let chat_id = chat.get_i64("id").unwrap_or(0);
    let chat_type = chat.get_str("type").unwrap_or("private");
    let topic_id = msg.get_i64("message_thread_id");
    let text = msg
        .get_str("text")
        .or_else(|| msg.get_str("caption"))
        .unwrap_or("")
        .trim()
        .to_string();

    if text.is_empty() {
        return;
    }

    if let Ok(mut guard) = state.0.lock() {
        guard.user_locales.insert(user_id, lang.clone());
    }

    // 1. Handle `/start` (Pairing & Collaborator Invites)
    if text.starts_with("/start") {
        let arg = text.split_whitespace().nth(1).unwrap_or("");
        if let Some(code) = arg.strip_prefix("pair_") {
            let paired = {
                let mut guard = state.0.lock().unwrap();
                if guard.active_pairing_code.as_deref() == Some(code) || guard.owner_user_id.is_none()
                {
                    guard.owner_user_id = Some(user_id);
                    guard.active_pairing_code = None;
                    true
                } else {
                    false
                }
            };
            if paired {
                save_owner_user_id(user_id, &cfg.device_token);
                let hostname = get_hostname();
                let body = t(
                    Some(&lang),
                    "bot.pair_success",
                    &[
                        ("device_name", &hostname),
                        ("user_id", &user_id.to_string()),
                    ],
                );
                let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &body, None);
                send_control_center(cfg, state, chat_id, topic_id, user_id, &lang);
            } else {
                let body = t(Some(&lang), "bot.pair_invalid", &[]);
                let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &body, None);
            }
            return;
        }

        if let Some(inv_tok) = arg.strip_prefix("invite_") {
            let accepted = {
                let mut guard = state.0.lock().unwrap();
                if let Some((proj_name, role)) = guard.invite_tokens.get(inv_tok).cloned() {
                    guard.collaborators.push(CollaboratorEntry {
                        project_name: proj_name.clone(),
                        user_id,
                        username: username.clone(),
                        role: role.clone(),
                        max_permission_mode: if role == "executor" {
                            "auto_edit".into()
                        } else {
                            "ask".into()
                        },
                    });
                    Some((proj_name, role))
                } else {
                    None
                }
            };
            if let Some((proj_name, role)) = accepted {
                let role_label = t(Some(&lang), &format!("roles.{}", role), &[]);
                let msg_txt = t(
                    Some(&lang),
                    "bot.invite_accepted",
                    &[("role", &role_label), ("project", &proj_name)],
                );
                let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &msg_txt, None);
            } else {
                let _ = send_telegram_html(
                    &cfg.bot_token,
                    chat_id,
                    topic_id,
                    &t(Some(&lang), "bot.pair_invalid", &[]),
                    None,
                );
            }
            return;
        }

        let _ = sync_local_projects_and_topics(cfg, state);
        send_control_center(cfg, state, chat_id, topic_id, user_id, &lang);
        return;
    }

    if text == "/control" || text == "/projects" || text == "/settings" || text == "/sync" {
        let _ = sync_local_projects_and_topics(cfg, state);
        let _ = sync_local_projects_and_topics(cfg, state);
        send_control_center(cfg, state, chat_id, topic_id, user_id, &lang);
        return;
    }

    if text == "/setup_group" && (chat_type == "supergroup" || chat_type == "group") {
        let is_owner = state.0.lock().unwrap().owner_user_id == Some(user_id);
        if !is_owner {
            let _ = send_telegram_html(
                &cfg.bot_token,
                chat_id,
                topic_id,
                &t(Some(&lang), "bot.group_not_owner", &[]),
                None,
            );
            return;
        }
        verify_and_configure_group(cfg, state, chat_id, &lang);
        return;
    }

    // 2. Check if user is replying to an active Interactive Question with custom free text
    {
        let (lock, cvar) = &**state;
        let mut guard = lock.lock().unwrap();
        for q in guard.questions.values_mut() {
            if !q.resolved {
                q.resolved = true;
                q.allow = true;
                q.custom_reply = Some(text.clone());
                q.chosen_labels = vec![text.clone()];
                cvar.notify_all();
                let _ = send_telegram_html(
                    &cfg.bot_token,
                    chat_id,
                    topic_id,
                    "✅ <i>Custom response sent to Harness.</i>",
                    None,
                );
                return;
            }
        }
    }

    // 3. Determine User's Role (`owner` | `executor` | `proposer` | `viewer`)
    let project_name = current_project_name();
    let user_role = resolve_user_role(state, user_id, &project_name, chat_id, chat_type);

    if user_role == "viewer" {
        if chat_type == "private" || text.contains(&cfg.bot_name) {
            let _ = send_telegram_html(
                &cfg.bot_token,
                chat_id,
                topic_id,
                &t(Some(&lang), "bot.viewer_denied", &[]),
                None,
            );
        }
        return;
    }

    // 4. Resolve or create Conversation Session for this Topic
    let session = get_or_create_topic_session(state, chat_id, topic_id, &project_name);
    let spec = build_prompt_spec_with_flags(&text, &session, &user_role);

    // 5. If user is `proposer`, hold prompt and send Approval Card to the Owner!
    if user_role == "proposer" {
        let owner_id = {
            let mut guard = state.0.lock().unwrap();
            let req_id = format!("ownprm_{}", now_millis() % 1_000_000);
            guard.pending_owner_prompts.insert(
                req_id.clone(),
                PendingOwnerPrompt {
                    id: req_id.clone(),
                    spec: spec.clone(),
                    project_name: project_name.clone(),
                    topic_title: session.title.clone(),
                    author_id: user_id,
                    author_name: username.clone(),
                    origin_chat_id: chat_id,
                    origin_topic_id: topic_id,
                },
            );
            (guard.owner_user_id, req_id)
        };

        let _ = send_telegram_html(
            &cfg.bot_token,
            chat_id,
            topic_id,
            &t(
                Some(&lang),
                "bot.prompt_held_for_approval",
                &[("project", &project_name)],
            ),
            None,
        );

        if let (Some(oid), req_id) = owner_id {
            let card = t(
                Some(&lang),
                "bot.owner_approval_card",
                &[
                    ("author", &escape_html(&username)),
                    ("project", &escape_html(&project_name)),
                    ("topic", &escape_html(&session.title)),
                    ("model", &spec.model),
                    ("thinking", &spec.thinking_level),
                    ("permissions", &spec.permission_mode),
                    ("prompt", &escape_html(&spec.prompt)),
                ],
            );
            let kb = Json::obj(vec![(
                "inline_keyboard",
                Json::Array(vec![
                    Json::Array(vec![
                        Json::obj(vec![
                            (
                                "text",
                                Json::Str(t(Some(&lang), "buttons.owner_approve_run", &[])),
                            ),
                            (
                                "callback_data",
                                Json::Str(format!("own_apr:{}:approve", req_id)),
                            ),
                        ]),
                        Json::obj(vec![
                            ("text", Json::Str("📋 Plan-Only".into())),
                            (
                                "callback_data",
                                Json::Str(format!("own_apr:{}:plan", req_id)),
                            ),
                        ]),
                    ]),
                    Json::Array(vec![Json::obj(vec![
                        (
                            "text",
                            Json::Str(t(Some(&lang), "buttons.owner_reject", &[])),
                        ),
                        (
                            "callback_data",
                            Json::Str(format!("own_apr:{}:reject", req_id)),
                        ),
                    ])]),
                ]),
            )]);
            let _ = send_telegram_html(&cfg.bot_token, oid, None, &card, Some(kb));
        }
        return;
    }

    // 6. Owner or Executor: Dispatch immediately and notify quietly (Output-First when finished)
    let _ = dispatch_to_harness(&spec);
    let ack = t(
        Some(&lang),
        "bot.prompt_queued_owner",
        &[
            ("harness", &spec.harness),
            ("model", &spec.model),
            ("thinking", &spec.thinking_level),
            ("permissions", &spec.permission_mode),
        ],
    );
    let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &ack, None);
}

fn handle_callback_query(cfg: &BotConfig, state: &SharedBotState, cb: &Json) {
    let cb_id = cb.get_str("id").unwrap_or("");
    let data = cb.get_str("data").unwrap_or("");
    let from = match cb.get("from") {
        Some(f) => f,
        None => return,
    };
    let user_id = from.get_i64("id").unwrap_or(0);
    let lang = resolve_locale(from.get_str("language_code"));
    let msg = cb.get("message");
    let chat_id = msg
        .and_then(|m| m.get("chat"))
        .and_then(|c| c.get_i64("id"))
        .unwrap_or(0);
    let message_id = msg.and_then(|m| m.get_i64("message_id")).unwrap_or(0);
    let topic_id = msg.and_then(|m| m.get_i64("message_thread_id"));

    // 1. Group Re-check
    if let Some(grp_str) = data.strip_prefix("grp_recheck:") {
        if let Ok(gid) = grp_str.parse::<i64>() {
            verify_and_configure_group(cfg, state, gid, &lang);
        }
        let _ = answer_callback(&cfg.bot_token, cb_id, None);
        return;
    }

    // 2. Group Default Member Role change (Owner only)
    if let Some(rest) = data.strip_prefix("grp_role:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            let gid = parts[0].parse::<i64>().unwrap_or(0);
            let role = parts[1];
            let is_owner = state.0.lock().unwrap().owner_user_id == Some(user_id);
            if is_owner {
                state
                    .0
                    .lock()
                    .unwrap()
                    .group_default_roles
                    .insert(gid, role.to_string());
                let _ = answer_callback(
                    &cfg.bot_token,
                    cb_id,
                    Some(&format!("Group default role set to: {}", role)),
                );
            } else {
                let _ = answer_callback(
                    &cfg.bot_token,
                    cb_id,
                    Some(&t(Some(&lang), "bot.group_not_owner", &[])),
                );
            }
        }
        return;
    }

    // 3. Create New Conversation Topic (1 Project -> N Conversation Topics)
    if data == "new_topic" {
        let proj = current_project_name();
        let conv_id = format!("conv_{}", now_secs() % 100_000);
        let topic_title = format!("📁 [{}] 💬 Session #{}", proj, &conv_id[conv_id.len().saturating_sub(4)..]);

        let mut created_thread_id = topic_id;
        if let Ok(res) = telegram_api_call(
            &cfg.bot_token,
            "createForumTopic",
            &Json::obj(vec![
                ("chat_id", Json::Number(chat_id as f64)),
                ("name", Json::Str(topic_title.clone())),
            ]),
        ) {
            created_thread_id = res
                .get("result")
                .and_then(|r| r.get_i64("message_thread_id"))
                .or(topic_id);
        }

        let session = ConversationSession {
            conversation_id: conv_id.clone(),
            project_name: proj.clone(),
            workspace: env::current_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_default(),
            title: topic_title.clone(),
            owner_user_id: user_id,
            chat_id,
            topic_id: created_thread_id,
            harness: "antigravity".into(),
            model: "gemini-4-argon".into(),
            thinking_level: "medium".into(),
            permission_mode: "ask".into(),
        };

        {
            let mut guard = state.0.lock().unwrap();
            if let Some(tid) = created_thread_id {
                guard.topic_to_conv.insert((chat_id, tid), conv_id.clone());
            }
            guard.sessions.insert(conv_id.clone(), session.clone());
        }

        send_or_edit_topic_config_bar(cfg, chat_id, created_thread_id, None, &session, &lang);
        let _ = answer_callback(&cfg.bot_token, cb_id, Some(&topic_title));
        return;
    }

    // 4. Cycle Harness / Model / Thinking / Permissions on a Conversation Topic
    if let Some(rest) = data.strip_prefix("cfg:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            let conv_id = parts[0];
            let field = parts[1];
            let updated_session = {
                let mut guard = state.0.lock().unwrap();
                if let Some(sess) = guard.sessions.get_mut(conv_id) {
                    match field {
                        "harness" => {
                            let idx = HARNESS_LIST
                                .iter()
                                .position(|h| *h == sess.harness)
                                .unwrap_or(0);
                            sess.harness = HARNESS_LIST[(idx + 1) % HARNESS_LIST.len()].to_string();
                            sess.model = available_models_for_harness(&sess.harness)[0].to_string();
                        }
                        "model" => {
                            let models = available_models_for_harness(&sess.harness);
                            let idx = models.iter().position(|m| *m == sess.model).unwrap_or(0);
                            sess.model = models[(idx + 1) % models.len()].to_string();
                        }
                        "thinking" => {
                            let idx = THINKING_LEVELS
                                .iter()
                                .position(|t| *t == sess.thinking_level)
                                .unwrap_or(2);
                            sess.thinking_level =
                                THINKING_LEVELS[(idx + 1) % THINKING_LEVELS.len()].to_string();
                        }
                        "permissions" => {
                            let idx = PERMISSION_MODES
                                .iter()
                                .position(|p| *p == sess.permission_mode)
                                .unwrap_or(1);
                            sess.permission_mode =
                                PERMISSION_MODES[(idx + 1) % PERMISSION_MODES.len()].to_string();
                        }
                        _ => {}
                    }
                    Some(sess.clone())
                } else {
                    None
                }
            };
            if let Some(sess) = updated_session {
                send_or_edit_topic_config_bar(
                    cfg,
                    chat_id,
                    topic_id,
                    Some(message_id),
                    &sess,
                    &lang,
                );
            }
        }
        let _ = answer_callback(&cfg.bot_token, cb_id, None);
        return;
    }

    // 5. Generate Collaborator Invite Links (`viewer` | `proposer` | `executor`)
    if let Some(role) = data.strip_prefix("inv:") {
        let proj = current_project_name();
        let tok = format!("{}_{}", &proj[..proj.len().min(4)], now_millis() % 100_000);
        state
            .0
            .lock()
            .unwrap()
            .invite_tokens
            .insert(tok.clone(), (proj.clone(), role.to_string()));

        let clean_bot = cfg.bot_name.trim_start_matches('@');
        let link = format!("https://t.me/{}?start=invite_{}", clean_bot, tok);
        let role_label = t(Some(&lang), &format!("roles.{}", role), &[]);
        let txt = t(
            Some(&lang),
            "bot.invite_created",
            &[("project", &proj), ("role", &role_label), ("link", &link)],
        );
        let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &txt, None);
        let _ = answer_callback(&cfg.bot_token, cb_id, None);
        return;
    }

    // 6. Owner Approval for Proposer's Prompt (`own_apr:<id>:<action>`)
    if let Some(rest) = data.strip_prefix("own_apr:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            let req_id = parts[0];
            let action = parts[1];
            let pending = state.0.lock().unwrap().pending_owner_prompts.remove(req_id);
            if let Some(mut item) = pending {
                if action == "reject" {
                    let _ = send_telegram_html(
                        &cfg.bot_token,
                        item.origin_chat_id,
                        item.origin_topic_id,
                        &format!("❌ Prompt rejected by Owner ({}).", item.author_name),
                        None,
                    );
                } else {
                    if action == "plan" {
                        item.spec.permission_mode = "plan".to_string();
                    }
                    let _ = dispatch_to_harness(&item.spec);
                    let _ = send_telegram_html(
                        &cfg.bot_token,
                        item.origin_chat_id,
                        item.origin_topic_id,
                        &format!(
                            "✅ Owner approved prompt from {} (Mode: {}). Executing...",
                            item.author_name, item.spec.permission_mode
                        ),
                        None,
                    );
                }
            }
        }
        let _ = answer_callback(&cfg.bot_token, cb_id, Some("Processed"));
        return;
    }

    // 7. Interactive Question Callbacks (`q_sel:`, `q_tog:`, `q_sub:`)
    if data.starts_with("q_sel:") || data.starts_with("q_tog:") || data.starts_with("q_sub:") {
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() >= 3 {
            let prefix = parts[0];
            let qid = parts[1];
            let val = parts[2];

            let (lock, cvar) = &**state;
            let mut guard = lock.lock().unwrap();
            if let Some(q) = guard.questions.get_mut(qid) {
                if prefix == "q_tog" {
                    let idx = val.parse::<usize>().unwrap_or(0);
                    if let Some(pos) = q.selected_indices.iter().position(|x| *x == idx) {
                        q.selected_indices.remove(pos);
                    } else {
                        q.selected_indices.push(idx);
                    }
                    // Update checklist buttons
                    let mut rows = Vec::new();
                    for (i, opt) in q.options.iter().enumerate() {
                        let mark = if q.selected_indices.contains(&i) {
                            "☑️"
                        } else {
                            "⬜"
                        };
                        rows.push(Json::Array(vec![Json::obj(vec![
                            ("text", Json::Str(format!("{} {}", mark, opt))),
                            ("callback_data", Json::Str(format!("q_tog:{}:{}", qid, i))),
                        ])]));
                    }
                    rows.push(Json::Array(vec![Json::obj(vec![
                        (
                            "text",
                            Json::Str(t(
                                Some(&lang),
                                "buttons.confirm_multi",
                                &[("count", &q.selected_indices.len().to_string())],
                            )),
                        ),
                        ("callback_data", Json::Str(format!("q_sub:{}:ok", qid))),
                    ])]));
                    let _ = telegram_api_call(
                        &cfg.bot_token,
                        "editMessageReplyMarkup",
                        &Json::obj(vec![
                            ("chat_id", Json::Number(chat_id as f64)),
                            ("message_id", Json::Number(message_id as f64)),
                            (
                                "reply_markup",
                                Json::obj(vec![("inline_keyboard", Json::Array(rows))]),
                            ),
                        ]),
                    );
                } else if prefix == "q_sub" {
                    q.resolved = true;
                    q.allow = true;
                    q.chosen_labels = q
                        .selected_indices
                        .iter()
                        .filter_map(|&i| q.options.get(i).cloned())
                        .collect();
                    cvar.notify_all();
                } else {
                    q.resolved = true;
                    if q.kind == "tool_approval" {
                        q.allow = val == "allow" || val == "allow_session";
                        q.chosen_labels = vec![val.to_string()];
                    } else {
                        let idx = val.parse::<usize>().unwrap_or(0);
                        q.allow = true;
                        if let Some(lbl) = q.options.get(idx) {
                            q.chosen_labels = vec![lbl.clone()];
                        }
                    }
                    cvar.notify_all();
                }
            }
        }
        let _ = answer_callback(&cfg.bot_token, cb_id, Some("✅ Sent to Harness"));
    }
}

fn send_control_center(
    cfg: &BotConfig,
    state: &SharedBotState,
    chat_id: i64,
    topic_id: Option<i64>,
    user_id: i64,
    lang: &str,
) {
    let proj = current_project_name();
    let sess = get_or_create_topic_session(state, chat_id, topic_id, &proj);
    let _ = user_id;

    let text = format!(
        "{}\n\n{}\n\n{}",
        t(Some(lang), "bot.welcome_title", &[]),
        t(Some(lang), "bot.welcome_body", &[]),
        t(Some(lang), "bot.control_center_body", &[])
    );

    let kb = Json::obj(vec![(
        "inline_keyboard",
        Json::Array(vec![
            Json::Array(vec![Json::obj(vec![
                (
                    "text",
                    Json::Str(t(Some(lang), "buttons.new_conversation", &[])),
                ),
                ("callback_data", Json::Str("new_topic".into())),
            ])]),
            Json::Array(vec![
                Json::obj(vec![
                    ("text", Json::Str("👁️ Invite Viewer".into())),
                    ("callback_data", Json::Str("inv:viewer".into())),
                ]),
                Json::obj(vec![
                    ("text", Json::Str("✋ Invite Proposer".into())),
                    ("callback_data", Json::Str("inv:proposer".into())),
                ]),
                Json::obj(vec![
                    ("text", Json::Str("🚀 Invite Executor".into())),
                    ("callback_data", Json::Str("inv:executor".into())),
                ]),
            ]),
        ]),
    )]);

    let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &text, Some(kb));
    send_or_edit_topic_config_bar(cfg, chat_id, topic_id, None, &sess, lang);
}

fn send_or_edit_topic_config_bar(
    cfg: &BotConfig,
    chat_id: i64,
    topic_id: Option<i64>,
    edit_message_id: Option<i64>,
    sess: &ConversationSession,
    lang: &str,
) {
    let thinking_label = t(
        Some(lang),
        &format!("thinking_levels.{}", sess.thinking_level),
        &[],
    );
    let perm_label = t(
        Some(lang),
        &format!("permission_modes.{}", sess.permission_mode),
        &[],
    );

    let text = t(
        Some(lang),
        "bot.topic_config_bar",
        &[
            ("project", &escape_html(&sess.project_name)),
            ("title", &escape_html(&sess.title)),
            ("harness", &sess.harness),
            ("model", &sess.model),
            ("thinking", &thinking_label),
            ("permissions", &perm_label),
        ],
    );

    let kb = Json::obj(vec![(
        "inline_keyboard",
        Json::Array(vec![
            Json::Array(vec![
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(
                            Some(lang),
                            "buttons.change_harness",
                            &[("value", &sess.harness)],
                        )),
                    ),
                    (
                        "callback_data",
                        Json::Str(format!("cfg:{}:harness", sess.conversation_id)),
                    ),
                ]),
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(
                            Some(lang),
                            "buttons.change_model",
                            &[("value", &sess.model)],
                        )),
                    ),
                    (
                        "callback_data",
                        Json::Str(format!("cfg:{}:model", sess.conversation_id)),
                    ),
                ]),
            ]),
            Json::Array(vec![
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(
                            Some(lang),
                            "buttons.change_thinking",
                            &[("value", &sess.thinking_level)],
                        )),
                    ),
                    (
                        "callback_data",
                        Json::Str(format!("cfg:{}:thinking", sess.conversation_id)),
                    ),
                ]),
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(
                            Some(lang),
                            "buttons.change_permissions",
                            &[("value", &sess.permission_mode)],
                        )),
                    ),
                    (
                        "callback_data",
                        Json::Str(format!("cfg:{}:permissions", sess.conversation_id)),
                    ),
                ]),
            ]),
        ]),
    )]);

    if let Some(mid) = edit_message_id {
        let _ = telegram_api_call(
            &cfg.bot_token,
            "editMessageText",
            &Json::obj(vec![
                ("chat_id", Json::Number(chat_id as f64)),
                ("message_id", Json::Number(mid as f64)),
                ("text", Json::Str(text)),
                ("parse_mode", Json::Str("HTML".into())),
                ("reply_markup", kb),
            ]),
        );
    } else {
        let _ = send_telegram_html(&cfg.bot_token, chat_id, topic_id, &text, Some(kb));
    }
}

/// Sends the Final Task Output to Telegram with an on-demand `<blockquote expandable>`
/// containing the Thinking & Steps summary (so no resources are wasted streaming live thoughts).
pub fn deliver_task_output_to_telegram(
    cfg: &BotConfig,
    state: &SharedBotState,
    summary: &TaskCompletionSummary,
) -> Result<(), String> {
    if cfg.mode == "cloud" && !cfg.cloud_url.contains("YOUR_TGCLOUD_APP_ID") {
        let steps_json: Vec<Json> = summary
            .steps_list
            .iter()
            .map(|s| Json::Str(s.clone()))
            .collect();
        let payload = Json::obj(vec![
            ("action", Json::Str("output".into())),
            ("conversationId", Json::Str(summary.conversation_id.clone())),
            ("projectName", Json::Str(summary.project_name.clone())),
            ("harness", Json::Str(summary.harness.clone())),
            ("outputText", Json::Str(summary.final_output.clone())),
            ("thinkingText", Json::Str(summary.thinking_summary.clone())),
            ("stepsSummary", Json::Array(steps_json)),
            (
                "filesTouchedCount",
                Json::Number(summary.files_touched_count as f64),
            ),
            ("gitDiff", Json::Str(summary.git_diff.clone())),
        ]);
        let endpoint = format!("{}/api/bridge", cfg.cloud_url.trim_end_matches('/'));
        let _ = https_post_json(&endpoint, &payload, Some(&cfg.device_token))?;
        return Ok(());
    }

    let (target_chat, mut target_topic, lang) = {
        let guard = state.0.lock().unwrap();
        let sess = guard.sessions.get(&summary.conversation_id);
        let chat = sess
            .map(|s| s.chat_id)
            .or(guard.owner_user_id)
            .unwrap_or(0);
        let topic = sess.and_then(|s| s.topic_id);
        let loc = guard
            .owner_user_id
            .and_then(|uid| guard.user_locales.get(&uid).cloned())
            .unwrap_or_else(|| cfg.default_locale.clone());
        (chat, topic, loc)
    };

    if target_topic.is_none() {
        if let Ok(raw) = fs::read_to_string(".thm/topics_map.json") {
            if let Some(j) = Json::parse_opt(&raw) {
                target_topic = j.get_i64(&summary.conversation_id);
            }
        }
    }

    if target_chat == 0 {
        return Err("No paired Owner chat ID yet. Run `/start` on the Telegram Bot.".into());
    }

    let header = t(
        Some(&lang),
        "bot.task_completed_header",
        &[
            ("project", &escape_html(&summary.project_name)),
            ("harness", &escape_html(&summary.harness)),
        ],
    );

    let mut expandable = String::new();
    if !summary.thinking_summary.is_empty() || !summary.steps_list.is_empty() {
        let exp_title = t(
            Some(&lang),
            "bot.steps_expandable_header",
            &[
                ("steps_count", &summary.steps_list.len().max(1).to_string()),
                ("files_count", &summary.files_touched_count.to_string()),
            ],
        );
        let steps_txt = summary
            .steps_list
            .iter()
            .take(12)
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, escape_html(s)))
            .collect::<Vec<_>>()
            .join("\n");
        let think_txt = if !summary.thinking_summary.is_empty() {
            format!(
                "\n\n💭 {}",
                escape_html(&truncate_str(&summary.thinking_summary, 700))
            )
        } else {
            String::new()
        };
        expandable = format!(
            "\n\n<blockquote expandable><b>{}</b>\n{}{}</blockquote>",
            exp_title, steps_txt, think_txt
        );
    }

    let full_html = format!(
        "{}\n\n{}{}",
        header,
        escape_html(&truncate_str(&summary.final_output, 2900)),
        expandable
    );

    send_telegram_html(&cfg.bot_token, target_chat, target_topic, &full_html, None)
}

/// Sends an Interactive Question (Single-Select, Multi-Select Checkboxes, or Tool Approval)
/// to Telegram and waits synchronously for the user's decision.
pub fn ask_interactive_on_telegram(
    cfg: &BotConfig,
    state: &SharedBotState,
    conv_id: &str,
    project_name: &str,
    kind: &str,
    tool_name: &str,
    question_text: &str,
    options: Vec<String>,
    timeout_secs: u64,
) -> Option<InteractiveQuestionState> {
    let qid = format!("q_{}", now_millis() % 1_000_000);
    let q_state = InteractiveQuestionState {
        id: qid.clone(),
        conversation_id: conv_id.to_string(),
        project_name: project_name.to_string(),
        kind: kind.to_string(),
        tool_name: tool_name.to_string(),
        question_text: question_text.to_string(),
        options: options.clone(),
        selected_indices: Vec::new(),
        resolved: false,
        allow: false,
        chosen_labels: Vec::new(),
        custom_reply: None,
    };

    let (target_chat, target_topic, lang) = {
        let mut guard = state.0.lock().unwrap();
        guard.questions.insert(qid.clone(), q_state);
        let sess = guard.sessions.get(conv_id);
        let chat = sess
            .map(|s| s.chat_id)
            .or(guard.owner_user_id)
            .unwrap_or(0);
        let topic = sess.and_then(|s| s.topic_id);
        let loc = guard
            .owner_user_id
            .and_then(|uid| guard.user_locales.get(&uid).cloned())
            .unwrap_or_else(|| cfg.default_locale.clone());
        (chat, topic, loc)
    };

    if cfg.mode == "direct" && target_chat != 0 {
        let mut rows = Vec::new();
        let header = if kind == "tool_approval" {
            rows.push(Json::Array(vec![
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(Some(&lang), "buttons.approve_once", &[])),
                    ),
                    ("callback_data", Json::Str(format!("q_sel:{}:allow", qid))),
                ]),
                Json::obj(vec![
                    (
                        "text",
                        Json::Str(t(Some(&lang), "buttons.approve_session", &[])),
                    ),
                    (
                        "callback_data",
                        Json::Str(format!("q_sel:{}:allow_session", qid)),
                    ),
                ]),
            ]));
            rows.push(Json::Array(vec![Json::obj(vec![
                ("text", Json::Str(t(Some(&lang), "buttons.reject", &[]))),
                ("callback_data", Json::Str(format!("q_sel:{}:deny", qid))),
            ])]));
            t(
                Some(&lang),
                "bot.permission_request_header",
                &[
                    ("tool", &escape_html(tool_name)),
                    ("project", &escape_html(project_name)),
                ],
            )
        } else if kind == "multi_select" {
            for (i, opt) in options.iter().enumerate() {
                rows.push(Json::Array(vec![Json::obj(vec![
                    ("text", Json::Str(format!("⬜ {}", opt))),
                    ("callback_data", Json::Str(format!("q_tog:{}:{}", qid, i))),
                ])]));
            }
            rows.push(Json::Array(vec![Json::obj(vec![
                (
                    "text",
                    Json::Str(t(
                        Some(&lang),
                        "buttons.confirm_multi",
                        &[("count", "0")],
                    )),
                ),
                ("callback_data", Json::Str(format!("q_sub:{}:ok", qid))),
            ])]));
            format!(
                "{}\n{}",
                t(
                    Some(&lang),
                    "bot.interactive_question_header",
                    &[
                        ("harness", "Harness"),
                        ("project", &escape_html(project_name)),
                        ("topic", conv_id),
                    ]
                ),
                t(Some(&lang), "bot.interactive_multi_hint", &[])
            )
        } else {
            for (i, opt) in options.iter().enumerate() {
                rows.push(Json::Array(vec![Json::obj(vec![
                    ("text", Json::Str(opt.clone())),
                    ("callback_data", Json::Str(format!("q_sel:{}:{}", qid, i))),
                ])]));
            }
            format!(
                "{}\n{}",
                t(
                    Some(&lang),
                    "bot.interactive_question_header",
                    &[
                        ("harness", "Harness"),
                        ("project", &escape_html(project_name)),
                        ("topic", conv_id),
                    ]
                ),
                t(Some(&lang), "bot.interactive_single_hint", &[])
            )
        };

        let html = format!("{}\n\n<blockquote>{}</blockquote>", header, escape_html(question_text));
        let kb = Json::obj(vec![("inline_keyboard", Json::Array(rows))]);
        let _ = send_telegram_html(&cfg.bot_token, target_chat, target_topic, &html, Some(kb));
    }

    // Wait on Condvar until user taps an option or replies on Telegram
    let (lock, cvar) = &**state;
    let mut guard = lock.lock().unwrap();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        if let Some(q) = guard.questions.get(&qid) {
            if q.resolved {
                let res = q.clone();
                guard.questions.remove(&qid);
                return Some(res);
            }
        }
        let (new_guard, timeout_res) = cvar.wait_timeout(guard, timeout).unwrap();
        guard = new_guard;
        if timeout_res.timed_out() {
            guard.questions.remove(&qid);
            return None;
        }
    }
}

fn resolve_user_role(
    state: &SharedBotState,
    user_id: i64,
    project_name: &str,
    chat_id: i64,
    chat_type: &str,
) -> String {
    let mut guard = state.0.lock().unwrap();
    if guard.owner_user_id.is_none() && chat_type == "private" {
        guard.owner_user_id = Some(user_id);
        return "owner".to_string();
    }
    if guard.owner_user_id == Some(user_id) {
        return "owner".to_string();
    }
    for col in &guard.collaborators {
        if col.user_id == user_id && (col.project_name == project_name || col.project_name == "*") {
            return col.role.clone();
        }
    }
    if chat_type == "supergroup" || chat_type == "group" {
        return guard
            .group_default_roles
            .get(&chat_id)
            .cloned()
            .unwrap_or_else(|| "viewer".to_string());
    }
    "viewer".to_string()
}

fn get_or_create_topic_session(
    state: &SharedBotState,
    chat_id: i64,
    topic_id: Option<i64>,
    project_name: &str,
) -> ConversationSession {
    let mut guard = state.0.lock().unwrap();
    let tid = topic_id.unwrap_or(0);
    if let Some(cid) = guard.topic_to_conv.get(&(chat_id, tid)).cloned() {
        if let Some(sess) = guard.sessions.get(&cid) {
            return sess.clone();
        }
    }

    let cid = format!("conv_{}_{}", chat_id.abs() % 10000, tid);
    let sess = ConversationSession {
        conversation_id: cid.clone(),
        project_name: project_name.to_string(),
        workspace: env::current_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        title: if tid > 0 {
            format!("Topic #{}", tid)
        } else {
            "Main Session".to_string()
        },
        owner_user_id: guard.owner_user_id.unwrap_or(0),
        chat_id,
        topic_id,
        harness: "antigravity".into(),
        model: "gemini-4-argon".into(),
        thinking_level: "medium".into(),
        permission_mode: "ask".into(),
    };
    guard.topic_to_conv.insert((chat_id, tid), cid.clone());
    guard.sessions.insert(cid, sess.clone());
    sess
}

fn build_prompt_spec_with_flags(
    raw_text: &str,
    sess: &ConversationSession,
    role: &str,
) -> PromptDispatchSpec {
    let mut harness = sess.harness.clone();
    let mut model = sess.model.clone();
    let mut thinking = sess.thinking_level.clone();
    let mut perms = sess.permission_mode.clone();

    let mut words: Vec<&str> = raw_text.split_whitespace().collect();
    while words.len() > 1 && words[0].starts_with('!') {
        let flag = words.remove(0).trim_start_matches('!').to_ascii_lowercase();
        if THINKING_LEVELS.contains(&flag.as_str()) {
            thinking = flag;
        } else if PERMISSION_MODES.contains(&flag.as_str()) {
            perms = flag;
        } else if HARNESS_LIST.contains(&flag.as_str()) {
            harness = flag;
            model = available_models_for_harness(&harness)[0].to_string();
        }
    }

    if role == "executor" && perms == "auto" {
        perms = "auto_edit".to_string();
    }

    PromptDispatchSpec {
        conversation_id: sess.conversation_id.clone(),
        workspace: sess.workspace.clone(),
        harness,
        model,
        thinking_level: thinking,
        permission_mode: perms,
        prompt: words.join(" "),
    }
}

pub fn send_telegram_html(
    bot_token: &str,
    chat_id: i64,
    topic_id: Option<i64>,
    html_text: &str,
    reply_markup: Option<Json>,
) -> Result<(), String> {
    let mut fields = vec![
        ("chat_id", Json::Number(chat_id as f64)),
        ("text", Json::Str(html_text.to_string())),
        ("parse_mode", Json::Str("HTML".into())),
    ];
    if let Some(tid) = topic_id {
        if tid > 0 {
            fields.push(("message_thread_id", Json::Number(tid as f64)));
        }
    }
    if let Some(rm) = reply_markup {
        fields.push(("reply_markup", rm));
    }
    telegram_api_call(bot_token, "sendMessage", &Json::obj(fields)).map(|_| ())
}

fn answer_callback(bot_token: &str, cb_id: &str, text: Option<&str>) -> Result<(), String> {
    let mut fields = vec![("callback_query_id", Json::Str(cb_id.to_string()))];
    if let Some(t) = text {
        fields.push(("text", Json::Str(t.to_string())));
    }
    telegram_api_call(bot_token, "answerCallbackQuery", &Json::obj(fields)).map(|_| ())
}

pub fn telegram_api_call(bot_token: &str, method: &str, payload: &Json) -> Result<Json, String> {
    if bot_token.is_empty() || bot_token == "YOUR_TELEGRAM_BOT_TOKEN_HERE" {
        return Err("TELEGRAM_BOT_TOKEN not configured".into());
    }
    let url = format!("https://api.telegram.org/bot{}/{}", bot_token, method);
    let raw = https_post_json(&url, payload, None)?;
    let parsed = Json::parse(&raw)?;
    if parsed.get_bool("ok") != Some(true) {
        let desc = parsed
            .get_str("description")
            .unwrap_or("Unknown Telegram API error");
        return Err(format!("Telegram API error ({}): {}", method, desc));
    }
    Ok(parsed)
}

pub fn https_post_json(
    url: &str,
    payload: &Json,
    bearer_token: Option<&str>,
) -> Result<String, String> {
    use std::io::Write;
    use std::process::Stdio;

    let body = payload.stringify();
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };

    let mut cmd = Command::new(curl_bin);
    cmd.args([
        "-s",
        "-X",
        "POST",
        url,
        "-H",
        "Content-Type: application/json; charset=utf-8",
    ]);
    if let Some(tok) = bearer_token {
        cmd.arg("-H").arg(format!("Authorization: Bearer {}", tok));
    }
    cmd.arg("--data-binary").arg("@-");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(body.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("HTTPS request failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn save_owner_user_id(user_id: i64, device_token: &str) {
    let _ = fs::create_dir_all(".thm");
    let info = Json::obj(vec![
        ("ownerUserId", Json::Number(user_id as f64)),
        ("deviceToken", Json::Str(device_token.to_string())),
        ("updatedAt", Json::Number(now_secs() as f64)),
    ]);
    let _ = fs::write(".thm/device.json", info.stringify());
}

pub fn current_project_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "telegram-harness-manager".to_string())
}

fn get_hostname() -> String {
    env::var("COMPUTERNAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Dev-Workstation".to_string())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
