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

use crate::json::Json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct PromptDispatchSpec {
    pub conversation_id: String,
    pub workspace: String,
    pub harness: String,
    pub model: String,
    pub thinking_level: String,
    pub permission_mode: String,
    pub prompt: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TaskCompletionSummary {
    pub conversation_id: String,
    pub project_name: String,
    pub workspace: String,
    pub harness: String,
    pub final_output: String,
    pub thinking_summary: String,
    pub steps_list: Vec<String>,
    pub files_touched_count: usize,
    pub git_diff: String,
}

#[derive(Clone, Debug)]
pub struct PromptTurn {
    pub index: usize,
    pub user_prompt: String,
    pub final_output: String,
    pub thinking_summary: String,
    pub tool_count: usize,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DiscoveredConversation {
    pub conversation_id: String,
    pub project_name: String,
    pub workspace: String,
    pub title: String,
    pub harness: String,
    pub archived: bool,
    pub prompts: Vec<PromptTurn>,
    pub last_user_prompt: String,
    pub last_output: String,
    pub last_thinking: String,
    pub step_count: usize,
    pub updated_at: u64,
}

pub fn available_models_for_harness(harness: &str) -> Vec<&'static str> {
    match harness {
        "antigravity" => vec!["gemini-4-argon", "claude-sonnet-4", "claude-opus-4", "gpt-5"],
        "claude-code" => vec!["claude-sonnet-4", "claude-opus-4", "claude-haiku-4"],
        "opencode" => vec![
            "anthropic/claude-sonnet-4",
            "google/gemini-2.5-pro",
            "openai/o3",
            "deepseek/deepseek-reasoner",
        ],
        "deepseek" => vec!["deepseek-reasoner", "deepseek-chat"],
        "aider" => vec!["sonnet", "deepseek", "o3-mini"],
        _ => vec!["default"],
    }
}

/// Scans `~/.gemini/antigravity/brain` and `~/.gemini/antigravity/annotations/*.pbtxt`
/// for Harness threads (both active and archived), ordered from most recently used to least recently used.
pub fn scan_local_conversations(max_count: usize) -> Vec<DiscoveredConversation> {
    let brain_dir = get_antigravity_brain_dir();
    let annotations_dir = get_antigravity_base_dir().join("annotations");
    let mut list: Vec<DiscoveredConversation> = Vec::new();

    let read_dir = match fs::read_dir(&brain_dir) {
        Ok(rd) => rd,
        Err(_) => return list,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let conv_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) if s != "tempmediaStorage" && !s.starts_with('.') => s.to_string(),
            _ => continue,
        };

        let transcript_path = path
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");

        if !transcript_path.exists() {
            continue;
        }

        let mtime_secs = fs::metadata(&transcript_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let pbtxt_path = annotations_dir.join(format!("{}.pbtxt", conv_id));
        let (pb_title, pb_archived, pb_last_view) = parse_antigravity_annotation(&pbtxt_path);

        // Exclude subagent trajectories that do not have a user-facing thread annotation in `annotations/*.pbtxt`
        let thread_title = match pb_title {
            Some(t) if !t.trim().is_empty() => t,
            _ => continue,
        };

        let updated_at = if pb_last_view > 0 {
            pb_last_view
        } else {
            mtime_secs
        };

        if let Some(mut disc) = parse_conversation_transcript(&conv_id, &transcript_path, updated_at) {
            disc.title = thread_title;
            disc.archived = pb_archived;
            list.push(disc);
        }
    }

    // Sort from most recently used to least recently used
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list.truncate(max_count);
    list
}

/// Parses `~/.gemini/antigravity/annotations/<conv_id>.pbtxt` for:
/// - `title:"..."`
/// - `archived:true`
/// - `last_user_view_time:{seconds:<u64> ...}`
fn parse_antigravity_annotation(pbtxt_path: &Path) -> (Option<String>, bool, u64) {
    let raw = match fs::read_to_string(pbtxt_path) {
        Ok(s) => s,
        Err(_) => return (None, false, 0),
    };

    let mut title: Option<String> = None;
    if let Some(pos) = raw.find("title:\"") {
        let rest = &raw[pos + 7..];
        let mut out = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next_c) = chars.next() {
                    out.push(next_c);
                }
            } else if c == '"' {
                break;
            } else {
                out.push(c);
            }
        }
        if !out.trim().is_empty() {
            title = Some(out.trim().to_string());
        }
    }

    let archived = raw.contains("archived:true") || raw.contains("archived: true");

    let mut last_view_secs = 0u64;
    if let Some(vpos) = raw.find("last_user_view_time") {
        let after = &raw[vpos..];
        if let Some(spos) = after.find("seconds:") {
            let num_part = after[spos + 8..].trim_start();
            let digits: String = num_part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(val) = digits.parse::<u64>() {
                last_view_secs = val;
            }
        }
    }

    (title, archived, last_view_secs)
}

fn parse_conversation_transcript(
    conv_id: &str,
    transcript_path: &Path,
    updated_at: u64,
) -> Option<DiscoveredConversation> {
    let content = fs::read_to_string(transcript_path).ok()?;
    let mut workspace = String::new();
    let mut first_prompt = String::new();
    let mut last_user_prompt = String::new();
    let mut last_output = String::new();
    let mut last_thinking = String::new();
    let mut step_count = 0usize;

    let mut prompts: Vec<PromptTurn> = Vec::new();
    let mut cur_prompt_text = String::new();
    let mut cur_output_text = String::new();
    let mut cur_thinking_chunks: Vec<String> = Vec::new();
    let mut cur_tool_count = 0usize;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        step_count += 1;

        if workspace.is_empty() {
            if let Some(ws) = extract_workspace_from_line(line) {
                workspace = ws;
            }
        }

        if let Some(step) = Json::parse_opt(line) {
            let stype = step.get_str("type").unwrap_or("");
            if stype == "USER_INPUT" {
                if let Some(raw_c) = step.get_str("content") {
                    let clean = clean_user_request_tags(raw_c);
                    // Ignore internal subagent wakeup notifications so only real user prompts become Pinned Turns
                    if !clean.is_empty() && !raw_c.contains("<SUBAGENT_NOTIFICATION>") {
                        if !cur_prompt_text.is_empty() {
                            let idx = prompts.len() + 1;
                            prompts.push(PromptTurn {
                                index: idx,
                                user_prompt: truncate_chars(&cur_prompt_text, 400),
                                final_output: if cur_output_text.is_empty() {
                                    "Completato.".to_string()
                                } else {
                                    truncate_chars(&cur_output_text, 750)
                                },
                                thinking_summary: if cur_thinking_chunks.is_empty() {
                                    "Esecuzione diretta completata.".to_string()
                                } else {
                                    truncate_chars(&cur_thinking_chunks.join("\n• "), 550)
                                },
                                tool_count: cur_tool_count,
                            });
                        }
                        if first_prompt.is_empty() {
                            first_prompt = clean.lines().next().unwrap_or(&clean).trim().to_string();
                        }
                        cur_prompt_text = clean.clone();
                        cur_output_text.clear();
                        cur_thinking_chunks.clear();
                        cur_tool_count = 0;
                        last_user_prompt = truncate_chars(&clean, 260);
                    }
                }
            } else if stype == "PLANNER_RESPONSE" {
                if let Some(c) = step.get_str("content") {
                    if !c.trim().is_empty() {
                        cur_output_text = c.trim().to_string();
                        last_output = truncate_chars(c.trim(), 600);
                    }
                }
                if let Some(th) = step.get_str("thinking") {
                    if !th.trim().is_empty() {
                        let short_th = truncate_chars(th.trim(), 220);
                        cur_thinking_chunks.push(short_th.clone());
                        last_thinking = truncate_chars(th.trim(), 400);
                    }
                }
                if let Some(tcs) = step.get_arr("tool_calls") {
                    cur_tool_count += tcs.len();
                }
            }
        }
    }

    if !cur_prompt_text.is_empty() {
        let idx = prompts.len() + 1;
        prompts.push(PromptTurn {
            index: idx,
            user_prompt: truncate_chars(&cur_prompt_text, 400),
            final_output: if cur_output_text.is_empty() {
                "In elaborazione / pronto.".to_string()
            } else {
                truncate_chars(&cur_output_text, 750)
            },
            thinking_summary: if cur_thinking_chunks.is_empty() {
                "Elaborazione contesto in corso.".to_string()
            } else {
                truncate_chars(&cur_thinking_chunks.join("\n• "), 550)
            },
            tool_count: cur_tool_count,
        });
    }

    if step_count == 0 || first_prompt.is_empty() {
        return None;
    }

    if workspace.is_empty() {
        let full_path = transcript_path.with_file_name("transcript_full.jsonl");
        if let Ok(full_raw) = fs::read_to_string(&full_path) {
            for line in full_raw.lines().take(50) {
                if let Some(ws) = extract_workspace_from_line(line) {
                    workspace = ws;
                    break;
                }
            }
        }
    }

    let project_name = if !workspace.is_empty() {
        Path::new(&workspace)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string()
    } else {
        "Antigravity".to_string()
    };

    let title = if !first_prompt.is_empty() {
        truncate_chars(&first_prompt, 48)
    } else {
        format!("Thread {}", &conv_id[..conv_id.len().min(8)])
    };

    Some(DiscoveredConversation {
        conversation_id: conv_id.to_string(),
        project_name,
        workspace,
        title,
        harness: "antigravity".to_string(),
        archived: false,
        prompts,
        last_user_prompt,
        last_output,
        last_thinking,
        step_count,
        updated_at,
    })
}

fn extract_workspace_from_line(line: &str) -> Option<String> {
    for marker in [
        "\\Dev\\",
        "/Dev/",
        "\\\\Dev\\\\",
        "github.com/RubensRainelli/",
    ] {
        if let Some(pos) = line.find(marker) {
            let after = &line[pos + marker.len()..];
            let proj: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                .collect();
            if !proj.is_empty() && proj.len() >= 2 {
                let home = env::var("USERPROFILE")
                    .or_else(|_| env::var("HOME"))
                    .unwrap_or_else(|_| "C:\\Users\\User".to_string());
                return Some(
                    PathBuf::from(home)
                        .join("Dev")
                        .join(proj)
                        .display()
                        .to_string(),
                );
            }
        }
    }
    None
}

fn clean_user_request_tags(raw: &str) -> String {
    if let (Some(s), Some(e)) = (raw.find("<USER_REQUEST>"), raw.find("</USER_REQUEST>")) {
        raw[s + 14..e].trim().to_string()
    } else {
        raw.trim().to_string()
    }
}

/// Dispatches a prompt to the selected AI Coding Harness (Antigravity, Claude Code, OpenCode, DeepSeek, Aider)
/// passing the configured Model, Thinking Level, and Permission Mode.
pub fn dispatch_to_harness(spec: &PromptDispatchSpec) -> Result<String, String> {
    save_to_local_inbox(spec);

    let effective_prompt = format!(
        "[THM Execution Context: harness={}, model={}, thinking={}, permissions={}]\n{}",
        spec.harness, spec.model, spec.thinking_level, spec.permission_mode, spec.prompt
    );

    match spec.harness.as_str() {
        "claude-code" => dispatch_claude_code(spec),
        "opencode" => dispatch_opencode(spec),
        "deepseek" | "aider" => dispatch_deepseek_or_aider(spec),
        _ => dispatch_antigravity(spec, &effective_prompt),
    }
}

fn dispatch_antigravity(spec: &PromptDispatchSpec, formatted_prompt: &str) -> Result<String, String> {
    let bin = if cfg!(target_os = "windows") {
        "agy.cmd"
    } else {
        "agy"
    };

    let mut cmd = Command::new(bin);
    if !spec.conversation_id.is_empty() && spec.conversation_id != "new" {
        cmd.args(["chat", "--reuse-window", "-m", formatted_prompt]);
    } else {
        cmd.args(["chat", "-m", formatted_prompt]);
    }
    if !spec.workspace.is_empty() && Path::new(&spec.workspace).exists() {
        cmd.current_dir(&spec.workspace);
    }

    match cmd.status() {
        Ok(st) if st.success() => Ok(format!(
            "Prompt dispatched to Antigravity ({} · thinking:{} · perms:{})",
            spec.model, spec.thinking_level, spec.permission_mode
        )),
        _ => Ok(format!(
            "Prompt queued in local Antigravity inbox ({} · thinking:{} · perms:{})",
            spec.model, spec.thinking_level, spec.permission_mode
        )),
    }
}

fn dispatch_claude_code(spec: &PromptDispatchSpec) -> Result<String, String> {
    let bin = if cfg!(target_os = "windows") {
        "claude.cmd"
    } else {
        "claude"
    };

    let claude_perm = match spec.permission_mode.as_str() {
        "plan" => "plan",
        "auto_edit" => "acceptEdits",
        "auto" => "bypassPermissions",
        _ => "default",
    };

    let mut cmd = Command::new(bin);
    cmd.args([
        "-p",
        &spec.prompt,
        "--model",
        &spec.model,
        "--permission-mode",
        claude_perm,
    ]);
    if !spec.workspace.is_empty() && Path::new(&spec.workspace).exists() {
        cmd.current_dir(&spec.workspace);
    }

    match cmd.spawn() {
        Ok(_) => Ok(format!(
            "Claude Code launched ({} · mode:{})",
            spec.model, claude_perm
        )),
        Err(e) => Err(format!("Claude Code CLI error: {}", e)),
    }
}

fn dispatch_opencode(spec: &PromptDispatchSpec) -> Result<String, String> {
    let bin = if cfg!(target_os = "windows") {
        "opencode.cmd"
    } else {
        "opencode"
    };

    let mut cmd = Command::new(bin);
    cmd.args(["run", &spec.prompt, "--model", &spec.model]);
    if !spec.workspace.is_empty() && Path::new(&spec.workspace).exists() {
        cmd.current_dir(&spec.workspace);
    }

    match cmd.spawn() {
        Ok(_) => Ok(format!("OpenCode started with model {}", spec.model)),
        Err(e) => Err(format!("OpenCode CLI error: {}", e)),
    }
}

fn dispatch_deepseek_or_aider(spec: &PromptDispatchSpec) -> Result<String, String> {
    let model_arg = if spec.model.starts_with("deepseek/") {
        spec.model.clone()
    } else {
        format!("deepseek/{}", spec.model)
    };

    let mut cmd = Command::new("aider");
    cmd.args(["--model", &model_arg, "--message", &spec.prompt, "--yes-always"]);
    if !spec.workspace.is_empty() && Path::new(&spec.workspace).exists() {
        cmd.current_dir(&spec.workspace);
    }

    match cmd.spawn() {
        Ok(_) => Ok(format!("DeepSeek / Aider started with {}", model_arg)),
        Err(e) => Err(format!("DeepSeek/Aider CLI error: {}", e)),
    }
}

fn save_to_local_inbox(spec: &PromptDispatchSpec) {
    let dir = get_antigravity_base_dir().join("thm_inbox");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("latest_telegram_prompt.json");
    let record = Json::obj(vec![
        ("timestamp", Json::Number(now_secs() as f64)),
        ("conversationId", Json::Str(spec.conversation_id.clone())),
        ("workspace", Json::Str(spec.workspace.clone())),
        ("harness", Json::Str(spec.harness.clone())),
        ("model", Json::Str(spec.model.clone())),
        ("thinkingLevel", Json::Str(spec.thinking_level.clone())),
        ("permissionMode", Json::Str(spec.permission_mode.clone())),
        ("prompt", Json::Str(spec.prompt.clone())),
    ]);
    let _ = fs::write(&path, record.stringify());
}

pub fn get_antigravity_base_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".gemini").join("antigravity")
}

pub fn get_antigravity_brain_dir() -> PathBuf {
    get_antigravity_base_dir().join("brain")
}

/// Extracts the final output, thinking steps, and git diff for a completed conversation turn
/// so the user gets ONE clean Output message on Telegram with on-demand Thinking & Diff access.
pub fn extract_completion_summary(conv_id: &str) -> Option<TaskCompletionSummary> {
    let brain_dir = get_antigravity_brain_dir();
    let conv_dir = brain_dir.join(conv_id);
    let transcript_path = conv_dir
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");

    let content = fs::read_to_string(&transcript_path).ok()?;
    let mut workspace = String::new();
    let mut final_output = String::new();
    let mut thinking_chunks: Vec<String> = Vec::new();
    let mut steps_list: Vec<String> = Vec::new();
    let mut files_touched: Vec<String> = Vec::new();

    for line in content.lines() {
        if workspace.is_empty() {
            if let Some(ws) = extract_workspace_from_line(line) {
                workspace = ws;
            }
        }

        let step = match Json::parse_opt(line) {
            Some(s) => s,
            None => continue,
        };
        let step_type = step.get_str("type").unwrap_or("");
        if step_type == "USER_INPUT" {
            thinking_chunks.clear();
            steps_list.clear();
        } else if step_type == "PLANNER_RESPONSE" {
            if let Some(txt) = step.get_str("content") {
                if !txt.trim().is_empty() {
                    final_output = txt.trim().to_string();
                }
            }
            if let Some(th) = step.get_str("thinking") {
                if !th.trim().is_empty() {
                    thinking_chunks.push(truncate_chars(th.trim(), 280));
                }
            }
            if let Some(tcs) = step.get_arr("tool_calls") {
                for tc in tcs {
                    let name = tc
                        .get_str("name")
                        .unwrap_or("tool")
                        .trim_start_matches("default_api:");
                    steps_list.push(format!("Tool: {}", name));
                    if name == "write_file" || name == "edit_file" {
                        if let Some(args) = tc.get("arguments").or_else(|| tc.get("args")) {
                            if let Some(p) = args.get_str("Path") {
                                if !files_touched.contains(&p.to_string()) {
                                    files_touched.push(p.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if workspace.is_empty() {
        let full_path = transcript_path.with_file_name("transcript_full.jsonl");
        if let Ok(full_raw) = fs::read_to_string(&full_path) {
            for line in full_raw.lines().take(50) {
                if let Some(ws) = extract_workspace_from_line(line) {
                    workspace = ws;
                    break;
                }
            }
        }
    }

    let project_name = if !workspace.is_empty() {
        Path::new(&workspace)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string()
    } else {
        "Antigravity".to_string()
    };

    let git_diff = capture_git_diff(&workspace);

    Some(TaskCompletionSummary {
        conversation_id: conv_id.to_string(),
        project_name,
        workspace,
        harness: "antigravity".to_string(),
        final_output: if final_output.is_empty() {
            "Task completed.".to_string()
        } else {
            final_output
        },
        thinking_summary: thinking_chunks.join("\n---\n"),
        steps_list,
        files_touched_count: files_touched.len(),
        git_diff,
    })
}

pub fn capture_git_diff(workspace: &str) -> String {
    let dir = if workspace.is_empty() { "." } else { workspace };
    if let Ok(out) = Command::new("git")
        .args(["diff", "--stat", "-p", "-U2"])
        .current_dir(dir)
        .output()
    {
        let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !txt.is_empty() {
            return truncate_chars(&txt, 3000);
        }
    }
    String::new()
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
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
