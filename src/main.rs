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

//! # Telegram Harness Manager (`telegram-harness-manager`)
//!
//! Universal Multi-Harness Skill, Telegram Bot & Serverless Bridge for:
//! - Google Antigravity (`agy`)
//! - Claude Code (`claude`)
//! - OpenCode (`opencode`)
//! - DeepSeek Harness / Aider (`deepseek` / `aider`)
//!
//! ## Subcommands:
//! - `telegram-harness-manager serve` (or no args): Starts the Telegram Bot / Cloud Bridge & Local Companion Server (`:8430`)
//! - `telegram-harness-manager pair`: Generates a cryptographic `device_token` & one-time pairing link
//! - `telegram-harness-manager notify --output "..."`: Sends final task output + expandable thinking block
//! - `telegram-harness-manager ask --question "..." --options "A|B|C"`: Interactive single/multi-select on Telegram
//! - `telegram-harness-manager invite --role viewer|proposer|executor`: Generates collaborator invite link
//! - `telegram-harness-manager hook pre-tool` / `hook stop`: Lifecycle hooks for Antigravity & Claude Code
//! - `telegram-harness-manager mcp`: Model Context Protocol (MCP) stdio server

mod harnesses;
mod i18n;
mod json;
mod pefl_guard;
mod telegram_bot;

use harnesses::{
    capture_git_diff, dispatch_to_harness, extract_completion_summary, PromptDispatchSpec,
    TaskCompletionSummary,
};
use json::Json;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use telegram_bot::{
    ask_interactive_on_telegram, current_project_name, deliver_task_output_to_telegram,
    generate_pairing_code, load_bot_config, new_shared_bot_state, spawn_telegram_worker,
    sync_local_projects_and_topics, BotConfig, SharedBotState,
};

const DEFAULT_PORT: u16 = 8430;

fn main() {
    let args: Vec<String> = env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("serve");

    // 1. Background Hook Mode (invoked by Antigravity / Claude Code): no interactive EULA prompt
    if subcmd == "hook" {
        let hook_kind = args.get(2).map(|s| s.as_str()).unwrap_or("");
        run_hook_client(hook_kind);
        return;
    }

    // 2. MCP Stdio Mode
    if subcmd == "mcp" {
        run_mcp_stdio_server();
        return;
    }

    // 3. Explicit PEFL Check / Acceptance Subcommands for AI Harnesses
    if subcmd == "pefl-check" {
        pefl_guard::print_pefl_banner();
        if pefl_guard::is_pefl_accepted(Path::new(".")) {
            println!("STATUS: PEFL_ACCEPTED (Valid within 7-day window)");
            std::process::exit(0);
        } else {
            println!("STATUS: PEFL_APPROVAL_REQUIRED");
            println!("Mostra il testo della licenza PEFL v1.0 qui sopra nella chat dell'Harness e chiedi conferma all'utente prima di eseguire `telegram-harness-manager pefl-accept`.");
            std::process::exit(42);
        }
    }

    if subcmd == "pefl-accept" {
        let remember = args.iter().any(|a| a == "--remember-7-days");
        let session_only = args.iter().any(|a| a == "--session-only");
        if !remember && !session_only {
            pefl_guard::print_pefl_banner();
            eprintln!("⚠️ Specificare `--remember-7-days` oppure `--session-only` SOLO dopo aver mostrato la licenza PEFL v1.0 nell'Harness e aver ricevuto il consenso esplicito dell'utente.");
            std::process::exit(1);
        }
        pefl_guard::save_pefl_acceptance(Path::new("."), remember);
        println!(
            "✅ Licenza PEFL v1.0 accettata dall'utente ({}).",
            if remember {
                "memorizzata per 7 giorni"
            } else {
                "valida per la sessione corrente"
            }
        );
        return;
    }

    // 4. Foreground Commands: Enforce PEFL v1.0 Clause 3 EULA startup check
    pefl_guard::verify_pefl_compliance();

    let cfg = load_bot_config();
    let state = new_shared_bot_state();

    match subcmd {
        "start" => run_detached_start(&cfg, &state),
        "stop" => run_stop_daemon(),
        "status" => run_status_command(),
        "sync" => {
            let count = sync_local_projects_and_topics(&cfg, &state);
            println!(
                "✅ Sincronizzati {} thread/conversazioni dei progetti attuali su Telegram Forum Topics.",
                count
            );
        }
        "pair" => run_pair_command(&cfg, &state),
        "notify" => run_notify_command(&cfg, &state, &args[2..]),
        "ask" => run_ask_command(&cfg, &state, &args[2..]),
        "invite" => run_invite_command(&cfg, &args[2..]),
        _ => run_daemon_server(cfg, state),
    }
}

/// Spawns `telegram-harness-manager serve` as a completely detached background OS process
/// (with `Stdio::null()` and `CREATE_NO_WINDOW | DETACHED_PROCESS` on Windows) and exits
/// immediately so NO task remains hanging or visible inside Antigravity / Claude Code!
fn run_detached_start(cfg: &BotConfig, state: &SharedBotState) {
    // First, sync local project topics immediately
    let synced = sync_local_projects_and_topics(cfg, state);

    // Check if daemon is already listening on DEFAULT_PORT
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(250)).is_ok() {
        println!(
            "✅ Telegram Harness Manager è già attivo in background sulla porta {} ({} conversazioni sincronizzate).",
            DEFAULT_PORT, synced
        );
        return;
    }

    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("telegram-harness-manager"));
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS (0x00000008) | CREATE_NO_WINDOW (0x08000000) | CREATE_NEW_PROCESS_GROUP (0x00000200)
        cmd.creation_flags(0x08000208);
    }

    match cmd.spawn() {
        Ok(child) => {
            let _ = fs::create_dir_all(".thm");
            let _ = fs::write(".thm/daemon.pid", child.id().to_string());
            println!(
                "🚀 Telegram Harness Manager avviato in background (PID: {}, Porta: {}, Topic sincronizzati: {}).",
                child.id(),
                DEFAULT_PORT,
                synced
            );
        }
        Err(e) => {
            eprintln!("❌ Impossibile avviare il demone in background: {}", e);
        }
    }
}

fn run_stop_daemon() {
    let _ = http_post_local("/api/internal/shutdown", "{}", 2);
    if let Ok(pid_str) = fs::read_to_string(".thm/daemon.pid") {
        let pid = pid_str.trim();
        if !pid.is_empty() {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", pid])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new("kill").arg(pid).output();
            }
        }
        let _ = fs::remove_file(".thm/daemon.pid");
    }
    println!("🛑 Demone Telegram Harness Manager arrestato.");
}

fn run_status_command() {
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(300)).is_ok() {
        println!("🟢 Telegram Harness Manager: ONLINE (Porta {})", DEFAULT_PORT);
    } else {
        println!("⚪ Telegram Harness Manager: OFFLINE");
    }
}

// ============================================================================
// CLI COMMANDS: PAIR, NOTIFY, ASK, INVITE
// ============================================================================

fn run_pair_command(cfg: &BotConfig, state: &SharedBotState) {
    let (code, link) = generate_pairing_code(cfg, state);
    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║  🔗 Telegram Harness Manager — Cryptographic Device Pairing              ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║  🤖 Bot:          {:<54} ║", cfg.bot_name);
    println!("║  🔑 Pairing Code: {:<54} ║", code);
    println!("║  🌐 Deep Link:    {:<54} ║", link);
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║  Open the link above in Telegram (or send `/start pair_{}` to {}) ║", code, cfg.bot_name);
    println!("║  Zero Bot Token exposure: your PC uses only its personal device_token.   ║");
    println!("╚══════════════════════════════════════════════════════════════════════════╝");
}

fn run_notify_command(cfg: &BotConfig, state: &SharedBotState, args: &[String]) {
    let project = parse_cli_flag(args, "--project").unwrap_or_else(current_project_name);
    let conv_id = parse_cli_flag(args, "--conversation").unwrap_or_else(|| "default".into());
    let harness = parse_cli_flag(args, "--harness").unwrap_or_else(|| "antigravity".into());
    let output = parse_cli_flag(args, "--output").unwrap_or_else(|| "Task completed.".into());
    let thinking = parse_cli_flag(args, "--thinking").unwrap_or_default();

    let summary = TaskCompletionSummary {
        conversation_id: conv_id,
        project_name: project,
        workspace: env::current_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        harness,
        final_output: output,
        thinking_summary: thinking,
        steps_list: vec!["Completed task execution".into()],
        files_touched_count: 0,
        git_diff: capture_git_diff("."),
    };

    match deliver_task_output_to_telegram(cfg, state, &summary) {
        Ok(_) => println!("✅ Final output delivered to Telegram."),
        Err(e) => eprintln!("⚠️ Delivery note: {}", e),
    }
}

fn run_ask_command(cfg: &BotConfig, state: &SharedBotState, args: &[String]) {
    let project = parse_cli_flag(args, "--project").unwrap_or_else(current_project_name);
    let conv_id = parse_cli_flag(args, "--conversation").unwrap_or_else(|| "default".into());
    let question = parse_cli_flag(args, "--question").unwrap_or_else(|| "Please choose an option:".into());
    let raw_opts = parse_cli_flag(args, "--options").unwrap_or_else(|| "Approve|Reject".into());
    let is_multi = parse_cli_flag(args, "--multi")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let options: Vec<String> = raw_opts
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let payload = Json::obj(vec![
        ("conversationId", Json::Str(conv_id)),
        ("projectName", Json::Str(project)),
        (
            "kind",
            Json::Str(if is_multi { "multi_select" } else { "single_select" }.into()),
        ),
        ("questionText", Json::Str(question)),
        (
            "options",
            Json::Array(options.into_iter().map(Json::Str).collect()),
        ),
    ]);

    if let Ok(resp) = http_post_local("/api/internal/ask", &payload.stringify(), 240) {
        println!("{}", resp);
    } else {
        let _ = (cfg, state);
        println!("{{\"ok\":false,\"error\":\"Local daemon not running on port 8430\"}}");
    }
}

fn run_invite_command(cfg: &BotConfig, args: &[String]) {
    let project = parse_cli_flag(args, "--project").unwrap_or_else(current_project_name);
    let role = parse_cli_flag(args, "--role").unwrap_or_else(|| "viewer".into());
    let clean_bot = cfg.bot_name.trim_start_matches('@');
    let token = format!(
        "{}_{}_{}",
        &project[..project.len().min(4)],
        role,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 100_000)
            .unwrap_or(1)
    );
    let link = format!("https://t.me/{}?start=invite_{}", clean_bot, token);
    println!("🔗 Collaborator Invite Link ({} on {}):\n{}", role, project, link);
}

// ============================================================================
// HOOK CLIENT (`telegram-harness-manager hook pre-tool` / `hook stop`)
// ============================================================================

fn run_hook_client(hook_kind: &str) {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    let endpoint = match hook_kind {
        "pre-tool" => "/api/internal/hook/pre-tool",
        "stop" => "/api/internal/hook/stop",
        _ => return,
    };

    if let Ok(resp) = http_post_local(endpoint, &input, 230) {
        if !resp.trim().is_empty() {
            println!("{}", resp.trim());
        }
    }
}

fn http_post_local(path: &str, body: &str, timeout_secs: u64) -> Result<String, std::io::Error> {
    let addr = format!("127.0.0.1:{}", DEFAULT_PORT);
    let mut stream = TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(600))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(timeout_secs)));

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        DEFAULT_PORT,
        body.len(),
        body
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    if let Some(idx) = raw.find("\r\n\r\n") {
        Ok(raw[idx + 4..].to_string())
    } else {
        Ok(String::new())
    }
}

// ============================================================================
// DAEMON SERVER (`:8430`) + TELEGRAM BOT / SERVERLESS BRIDGE
// ============================================================================

fn run_daemon_server(cfg: BotConfig, state: SharedBotState) {
    let (pair_code, pair_link) = generate_pairing_code(&cfg, &state);

    println!("╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║  🚀 Telegram Harness Manager v1.0.0 — Active on Port {:<19}║", DEFAULT_PORT);
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║  🤖 Telegram Bot: {:<54} ║", cfg.bot_name);
    println!("║  ⚙️  Runtime Mode: {:<54} ║", cfg.mode.to_uppercase());
    println!("║  🔗 Pairing Link: {:<54} ║", pair_link);
    println!("║  🔑 Pairing Code: {:<54} ║", format!("/start pair_{}", pair_code));
    println!("║  🌐 Mini App UI:  http://127.0.0.1:{:<37} ║", DEFAULT_PORT);
    println!("╚══════════════════════════════════════════════════════════════════════════╝\n");

    spawn_telegram_worker(cfg.clone(), Arc::clone(&state));

    let bind_addr = format!("0.0.0.0:{}", DEFAULT_PORT);
    let listener = TcpListener::bind(&bind_addr).unwrap_or_else(|e| {
        eprintln!("❌ Cannot bind {}: {}", bind_addr, e);
        std::process::exit(1);
    });

    for stream in listener.incoming().flatten() {
        let cfg_clone = cfg.clone();
        let st_clone = Arc::clone(&state);
        thread::spawn(move || {
            handle_http_client(stream, cfg_clone, st_clone);
        });
    }
}

fn handle_http_client(mut stream: TcpStream, cfg: BotConfig, state: SharedBotState) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut reader = match stream.try_clone() {
        Ok(s) => BufReader::new(s),
        Err(_) => return,
    };

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1].split('?').next().unwrap_or("/");

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
        if line.to_ascii_lowercase().starts_with("content-length:") {
            if let Some(v) = line.split(':').nth(1) {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    if method == "OPTIONS" {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let mut body = String::new();
    if content_length > 0 && content_length < 2_000_000 {
        let mut buf = vec![0u8; content_length];
        if reader.read_exact(&mut buf).is_ok() {
            body = String::from_utf8_lossy(&buf).to_string();
        }
    }

    match (method, path) {
        ("GET", "/api/state") => {
            let payload = build_api_state(&cfg, &state);
            send_json(&mut stream, 200, &payload);
        }
        ("POST", "/api/prompt") => {
            if let Some(j) = Json::parse_opt(&body) {
                let spec = PromptDispatchSpec {
                    conversation_id: j.get_str("conversationId").unwrap_or("default").to_string(),
                    workspace: env::current_dir()
                        .map(|d| d.display().to_string())
                        .unwrap_or_default(),
                    harness: j.get_str("harness").unwrap_or("antigravity").to_string(),
                    model: j.get_str("model").unwrap_or("gemini-4-argon").to_string(),
                    thinking_level: j.get_str("thinkingLevel").unwrap_or("medium").to_string(),
                    permission_mode: j.get_str("permissionMode").unwrap_or("ask").to_string(),
                    prompt: j.get_str("prompt").unwrap_or("").to_string(),
                };
                let _ = dispatch_to_harness(&spec);
            }
            send_json(&mut stream, 200, &Json::obj(vec![("ok", Json::Bool(true))]));
        }
        ("POST", "/api/internal/ask") => {
            handle_internal_ask(&mut stream, &cfg, &state, &body);
        }
        ("POST", "/api/internal/hook/pre-tool") => {
            handle_hook_pre_tool(&mut stream, &cfg, &state, &body);
        }
        ("POST", "/api/internal/hook/stop") => {
            handle_hook_stop(&mut stream, &cfg, &state, &body);
        }
        ("POST", "/api/internal/shutdown") => {
            send_json(&mut stream, 200, &Json::obj(vec![("ok", Json::Bool(true))]));
            std::process::exit(0);
        }
        ("GET", _) => {
            serve_static_webapp(&mut stream, path);
        }
        _ => {
            send_json(
                &mut stream,
                404,
                &Json::obj(vec![("error", Json::Str("Not Found".into()))]),
            );
        }
    }
}

fn build_api_state(cfg: &BotConfig, state: &SharedBotState) -> Json {
    let guard = state.0.lock().unwrap();
    let convs: Vec<Json> = guard
        .sessions
        .values()
        .map(|s| {
            Json::obj(vec![
                ("id", Json::Str(s.conversation_id.clone())),
                ("projectName", Json::Str(s.project_name.clone())),
                ("title", Json::Str(s.title.clone())),
                ("harness", Json::Str(s.harness.clone())),
                ("model", Json::Str(s.model.clone())),
                ("thinking_level", Json::Str(s.thinking_level.clone())),
                ("permission_mode", Json::Str(s.permission_mode.clone())),
                ("status", Json::Str("IDLE".into())),
            ])
        })
        .collect();

    let cols: Vec<Json> = guard
        .collaborators
        .iter()
        .map(|c| {
            Json::obj(vec![
                ("projectName", Json::Str(c.project_name.clone())),
                ("username", Json::Str(c.username.clone())),
                ("role", Json::Str(c.role.clone())),
            ])
        })
        .collect();

    Json::obj(vec![
        ("ok", Json::Bool(true)),
        ("botName", Json::Str(cfg.bot_name.clone())),
        ("mode", Json::Str(cfg.mode.clone())),
        ("conversations", Json::Array(convs)),
        ("collaborators", Json::Array(cols)),
    ])
}

fn handle_internal_ask(
    stream: &mut TcpStream,
    cfg: &BotConfig,
    state: &SharedBotState,
    body: &str,
) {
    let parsed = match Json::parse_opt(body) {
        Some(j) => j,
        None => {
            send_json(stream, 400, &Json::obj(vec![("ok", Json::Bool(false))]));
            return;
        }
    };
    let conv_id = parsed.get_str("conversationId").unwrap_or("default");
    let proj = parsed.get_str("projectName").unwrap_or("Project");
    let kind = parsed.get_str("kind").unwrap_or("single_select");
    let q_text = parsed.get_str("questionText").unwrap_or("");
    let opts: Vec<String> = parsed
        .get_arr("options")
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if let Some(res) =
        ask_interactive_on_telegram(cfg, state, conv_id, proj, kind, "ask_question", q_text, opts, 210)
    {
        let chosen: Vec<Json> = res.chosen_labels.into_iter().map(Json::Str).collect();
        send_json(
            stream,
            200,
            &Json::obj(vec![
                ("ok", Json::Bool(true)),
                ("allow", Json::Bool(res.allow)),
                ("selected", Json::Array(chosen)),
            ]),
        );
    } else {
        send_json(
            stream,
            200,
            &Json::obj(vec![("ok", Json::Bool(false)), ("timeout", Json::Bool(true))]),
        );
    }
}

/// Handles `PreToolUse` hook from Antigravity / Claude Code:
/// - Supports `ask_question` with multiple-choice options (`single_select` / `multi_select`)
/// - Enforces conversation `permission_mode` (`plan` blocks writes/commands, `ask` prompts on Telegram,
///   `auto_edit` allows file edits and asks for commands, `auto` allows all)
fn handle_hook_pre_tool(
    stream: &mut TcpStream,
    cfg: &BotConfig,
    state: &SharedBotState,
    body: &str,
) {
    // Clause 3 PEFL v1.0 Enforcement inside Harness Hook:
    // If the user has not accepted PEFL v1.0 yet (or 7 days expired), instruct the Harness
    // to display the full PEFL v1.0 license in the chat and ask the user for approval!
    if !pefl_guard::is_pefl_accepted(Path::new(".")) {
        let reason = format!(
            "{}\n\n⚠️ [PEFL v1.0 APPROVAL REQUIRED IN HARNESS]\n\
            Prima di procedere, mostra OBBLIGATORIAMENTE il testo della licenza PEFL v1.0 qui sopra nella chat dell'Harness \
            e chiedi all'utente (con `ask_question`) se accetta i termini e se vuole ricordarla per 7 giorni.\n\
            Dopo il consenso dell'utente, esegui `telegram-harness-manager pefl-accept --remember-7-days` (o `--session-only`).",
            pefl_guard::pefl_license_foreground_text()
        );
        let out = Json::obj(vec![(
            "hookSpecificOutput",
            Json::obj(vec![
                ("hookEventName", Json::Str("PreToolUse".into())),
                ("permissionDecision", Json::Str("deny".into())),
                ("permissionDecisionReason", Json::Str(reason)),
            ]),
        )]);
        send_json(stream, 200, &out);
        return;
    }

    let parsed = match Json::parse_opt(body) {
        Some(j) => j,
        None => {
            send_json(stream, 200, &Json::Object(BTreeMap::new()));
            return;
        }
    };

    let tool_name = parsed.get_str("tool_name").unwrap_or("unknown");
    let conv_id = parsed.get_str("conversation_id").unwrap_or("default");
    let cwd = parsed.get_str("cwd").unwrap_or("");
    let project_name = if !cwd.is_empty() {
        Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string()
    } else {
        current_project_name()
    };

    let perm_mode = {
        let guard = state.0.lock().unwrap();
        guard
            .sessions
            .get(conv_id)
            .map(|s| s.permission_mode.clone())
            .unwrap_or_else(|| "ask".to_string())
    };

    // 1. In `plan` mode, block any mutating tool or command automatically
    if perm_mode == "plan"
        && (tool_name == "run_command"
            || tool_name == "Bash"
            || tool_name == "edit_file"
            || tool_name == "write_file")
    {
        let out = Json::obj(vec![(
            "hookSpecificOutput",
            Json::obj(vec![
                ("hookEventName", Json::Str("PreToolUse".into())),
                ("permissionDecision", Json::Str("deny".into())),
                (
                    "permissionDecisionReason",
                    Json::Str("Conversation is in Plan (Read-Only) mode on Telegram.".into()),
                ),
            ]),
        )]);
        send_json(stream, 200, &out);
        return;
    }

    // 2. In `auto` mode (or `auto_edit` for file edits), allow without interrupting the user
    if perm_mode == "auto"
        || (perm_mode == "auto_edit" && (tool_name == "edit_file" || tool_name == "write_file"))
    {
        send_json(stream, 200, &Json::Object(BTreeMap::new()));
        return;
    }

    let tool_input = parsed.get("tool_input");

    // 3. If the tool is `ask_question`, forward its exact options (single or multi-select) to Telegram!
    if tool_name.contains("ask_question") {
        if let Some(q_arr) = tool_input.and_then(|ti| ti.get_arr("questions")) {
            if let Some(first_q) = q_arr.first() {
                let q_text = first_q.get_str("question").unwrap_or("Choose an option:");
                let is_multi = first_q.get_bool("is_multi_select").unwrap_or(false);
                let opts: Vec<String> = first_q
                    .get_arr("options")
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let _ = ask_interactive_on_telegram(
                    cfg,
                    state,
                    conv_id,
                    &project_name,
                    if is_multi { "multi_select" } else { "single_select" },
                    "ask_question",
                    q_text,
                    opts,
                    180,
                );
            }
        }
        send_json(stream, 200, &Json::Object(BTreeMap::new()));
        return;
    }

    // 4. Ask Tool Approval on Telegram (`run_command`, `Bash`, `edit_file`, `write_file`)
    let summary = tool_input
        .and_then(|ti| {
            ti.get_str("CommandLine")
                .or_else(|| ti.get_str("command"))
                .or_else(|| ti.get_str("Path"))
        })
        .unwrap_or(tool_name);

    if let Some(res) = ask_interactive_on_telegram(
        cfg,
        state,
        conv_id,
        &project_name,
        "tool_approval",
        tool_name,
        summary,
        vec![],
        180,
    ) {
        let decision_str = if res.allow { "allow" } else { "deny" };
        let out = Json::obj(vec![(
            "hookSpecificOutput",
            Json::obj(vec![
                ("hookEventName", Json::Str("PreToolUse".into())),
                ("permissionDecision", Json::Str(decision_str.into())),
                (
                    "permissionDecisionReason",
                    Json::Str("Decided via Telegram Harness Manager".into()),
                ),
            ]),
        )]);
        send_json(stream, 200, &out);
    } else {
        send_json(stream, 200, &Json::Object(BTreeMap::new()));
    }
}

/// Handles `Stop` hook when the agent finishes a turn:
/// Extracts the final output, thinking steps, and git diff, and delivers ONE clean message to Telegram.
fn handle_hook_stop(
    stream: &mut TcpStream,
    cfg: &BotConfig,
    state: &SharedBotState,
    body: &str,
) {
    let conv_id = Json::parse_opt(body)
        .and_then(|j| j.get_str("conversation_id").map(|s| s.to_string()))
        .unwrap_or_else(|| "default".to_string());

    let summary = extract_completion_summary(&conv_id).unwrap_or_else(|| TaskCompletionSummary {
        conversation_id: conv_id,
        project_name: current_project_name(),
        workspace: String::new(),
        harness: "antigravity".into(),
        final_output: "Task completed and awaiting your next instruction.".into(),
        thinking_summary: String::new(),
        steps_list: vec![],
        files_touched_count: 0,
        git_diff: capture_git_diff("."),
    });

    let _ = deliver_task_output_to_telegram(cfg, state, &summary);
    send_json(stream, 200, &Json::Object(BTreeMap::new()));
}

// ============================================================================
// MCP SERVER MODE (`telegram-harness-manager mcp`)
// ============================================================================

fn run_mcp_stdio_server() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let cfg = load_bot_config();
    let state = new_shared_bot_state();

    for line in stdin.lock().lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }
        let req = match Json::parse_opt(&line) {
            Some(j) => j,
            None => continue,
        };
        let id = req.get("id").cloned().unwrap_or(Json::Null);
        let method = req.get_str("method").unwrap_or("");

        let resp = match method {
            "initialize" => Json::obj(vec![
                ("jsonrpc", Json::Str("2.0".into())),
                ("id", id),
                (
                    "result",
                    Json::obj(vec![
                        ("protocolVersion", Json::Str("2024-11-05".into())),
                        (
                            "serverInfo",
                            Json::obj(vec![
                                ("name", Json::Str("telegram-harness-manager".into())),
                                ("version", Json::Str("1.0.0".into())),
                            ]),
                        ),
                        ("capabilities", Json::obj(vec![("tools", Json::obj(vec![]))])),
                    ]),
                ),
            ]),
            "tools/list" => Json::obj(vec![
                ("jsonrpc", Json::Str("2.0".into())),
                ("id", id),
                (
                    "result",
                    Json::obj(vec![(
                        "tools",
                        Json::Array(vec![
                            Json::obj(vec![
                                ("name", Json::Str("telegram_notify_output".into())),
                                (
                                    "description",
                                    Json::Str("Send final task output and expandable thinking summary to the conversation's Telegram Topic".into()),
                                ),
                            ]),
                            Json::obj(vec![
                                ("name", Json::Str("telegram_ask_options".into())),
                                (
                                    "description",
                                    Json::Str("Ask the user a single-select or multi-select question on Telegram and return their chosen option(s)".into()),
                                ),
                            ]),
                        ]),
                    )]),
                ),
            ]),
            "tools/call" => {
                let params = req.get("params");
                let tool_name = params.and_then(|p| p.get_str("name")).unwrap_or("");
                let args = params.and_then(|p| p.get("arguments"));
                let text_res = if tool_name == "telegram_notify_output" {
                    let out_txt = args
                        .and_then(|a| a.get_str("output"))
                        .unwrap_or("Task completed.")
                        .to_string();
                    let think_txt = args
                        .and_then(|a| a.get_str("thinking"))
                        .unwrap_or("")
                        .to_string();
                    let summary = TaskCompletionSummary {
                        conversation_id: "default".into(),
                        project_name: current_project_name(),
                        workspace: String::new(),
                        harness: "antigravity".into(),
                        final_output: out_txt,
                        thinking_summary: think_txt,
                        steps_list: vec![],
                        files_touched_count: 0,
                        git_diff: String::new(),
                    };
                    let _ = deliver_task_output_to_telegram(&cfg, &state, &summary);
                    "Delivered final output to Telegram.".to_string()
                } else {
                    "Executed Telegram Harness Manager tool.".to_string()
                };

                Json::obj(vec![
                    ("jsonrpc", Json::Str("2.0".into())),
                    ("id", id),
                    (
                        "result",
                        Json::obj(vec![(
                            "content",
                            Json::Array(vec![Json::obj(vec![
                                ("type", Json::Str("text".into())),
                                ("text", Json::Str(text_res)),
                            ])]),
                        )]),
                    ),
                ])
            }
            _ => continue,
        };

        let _ = writeln!(stdout, "{}", resp.stringify());
        let _ = stdout.flush();
    }
}

// ============================================================================
// STATIC MINI APP SERVER & HELPERS
// ============================================================================

fn serve_static_webapp(stream: &mut TcpStream, req_path: &str) {
    let clean = match req_path {
        "/" | "/index.html" => "index.html",
        p => p.trim_start_matches('/'),
    };
    if clean.contains("..") {
        send_json(stream, 403, &Json::obj(vec![("error", Json::Str("Forbidden".into()))]));
        return;
    }

    let candidates = [
        PathBuf::from("docs").join(clean),
        env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("docs").join(clean)))
            .unwrap_or_else(|| PathBuf::from("docs").join(clean)),
    ];

    for file_path in &candidates {
        if let Ok(bytes) = fs::read(file_path) {
            let mime = match file_path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "svg" => "image/svg+xml",
                _ => "application/octet-stream",
            };
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
                mime,
                bytes.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&bytes);
            return;
        }
    }

    send_json(stream, 404, &Json::obj(vec![("error", Json::Str("Not Found".into()))]));
}

fn send_json(stream: &mut TcpStream, status: u16, data: &Json) {
    let body = data.stringify();
    let resp = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

fn parse_cli_flag(args: &[String], flag: &str) -> Option<String> {
    for (i, a) in args.iter().enumerate() {
        if a == flag {
            return args.get(i + 1).cloned();
        }
    }
    None
}
