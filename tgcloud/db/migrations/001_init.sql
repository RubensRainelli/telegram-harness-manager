-- Personal and Educational Free License (PEF License / PEFL) v1.0
-- Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it)
-- Telegram Serverless (tgcloud) SQLite Schema - Multi-Tenant & Multi-Harness

CREATE TABLE IF NOT EXISTS users (
    telegram_user_id INTEGER PRIMARY KEY,
    username TEXT DEFAULT '',
    first_name TEXT DEFAULT '',
    language_code TEXT DEFAULT 'en',
    pefl_accepted_at INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    owner_user_id INTEGER NOT NULL,
    device_name TEXT NOT NULL,
    device_token_hash TEXT NOT NULL UNIQUE,
    os_name TEXT DEFAULT '',
    harnesses_json TEXT DEFAULT '["antigravity","claude-code","opencode","deepseek"]',
    last_seen_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(telegram_user_id)
);

CREATE TABLE IF NOT EXISTS pairing_codes (
    code TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    device_token_hash TEXT NOT NULL,
    os_name TEXT DEFAULT '',
    harnesses_json TEXT DEFAULT '[]',
    claimed_by_user_id INTEGER DEFAULT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    owner_user_id INTEGER NOT NULL,
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    workspace_path TEXT NOT NULL,
    default_harness TEXT DEFAULT 'antigravity',
    default_model TEXT DEFAULT 'gemini-4-argon',
    default_thinking TEXT DEFAULT 'medium',
    default_permission_mode TEXT DEFAULT 'ask',
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(telegram_user_id),
    FOREIGN KEY (device_id) REFERENCES devices(device_id)
);

-- 1 Project -> N Conversations (each mapped to its own Telegram Forum Topic)
CREATE TABLE IF NOT EXISTS conversations (
    conversation_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    owner_user_id INTEGER NOT NULL,
    device_id TEXT NOT NULL,
    title TEXT NOT NULL,
    harness TEXT NOT NULL DEFAULT 'antigravity',
    model TEXT NOT NULL DEFAULT 'gemini-4-argon',
    thinking_level TEXT NOT NULL DEFAULT 'medium',
    permission_mode TEXT NOT NULL DEFAULT 'ask',
    status TEXT NOT NULL DEFAULT 'IDLE',
    private_chat_id INTEGER DEFAULT NULL,
    private_topic_id INTEGER DEFAULT NULL,
    group_chat_id INTEGER DEFAULT NULL,
    group_topic_id INTEGER DEFAULT NULL,
    config_message_id INTEGER DEFAULT NULL,
    last_output TEXT DEFAULT '',
    last_thinking TEXT DEFAULT '',
    last_steps_json TEXT DEFAULT '[]',
    last_diff TEXT DEFAULT '',
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
);

-- Project Collaborators / Guests ACL
-- role: 'owner' | 'executor' | 'proposer' | 'viewer'
CREATE TABLE IF NOT EXISTS project_members (
    project_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    username TEXT DEFAULT '',
    role TEXT NOT NULL DEFAULT 'viewer',
    max_permission_mode TEXT NOT NULL DEFAULT 'auto_edit',
    invited_by INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (project_id, user_id),
    FOREIGN KEY (project_id) REFERENCES projects(project_id)
);

CREATE TABLE IF NOT EXISTS project_invites (
    invite_token TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    owner_user_id INTEGER NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer',
    max_permission_mode TEXT NOT NULL DEFAULT 'ask',
    expires_at INTEGER NOT NULL,
    used_by_user_id INTEGER DEFAULT NULL,
    created_at INTEGER NOT NULL
);

-- Group Forum Bindings (Only Owner of a paired Harness can link a group)
CREATE TABLE IF NOT EXISTS group_bindings (
    group_chat_id INTEGER PRIMARY KEY,
    project_id TEXT NOT NULL,
    owner_user_id INTEGER NOT NULL,
    default_member_role TEXT NOT NULL DEFAULT 'viewer',
    topics_enabled INTEGER NOT NULL DEFAULT 0,
    bot_can_manage_topics INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- Queued Prompts (supports Proposer -> Owner Manual Approval workflow)
CREATE TABLE IF NOT EXISTS prompt_queue (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    owner_user_id INTEGER NOT NULL,
    author_user_id INTEGER NOT NULL,
    author_name TEXT NOT NULL,
    prompt_text TEXT NOT NULL,
    harness TEXT NOT NULL,
    model TEXT NOT NULL,
    thinking_level TEXT NOT NULL,
    permission_mode TEXT NOT NULL,
    approval_status TEXT NOT NULL DEFAULT 'approved', -- 'approved' | 'pending_owner' | 'rejected'
    execution_status TEXT NOT NULL DEFAULT 'queued',  -- 'queued' | 'running' | 'completed' | 'failed'
    created_at INTEGER NOT NULL
);

-- Interactive Questions (Single-Select, Multi-Select Checkboxes, Tool Approvals)
CREATE TABLE IF NOT EXISTS interactive_questions (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    owner_user_id INTEGER NOT NULL,
    kind TEXT NOT NULL, -- 'single_select' | 'multi_select' | 'tool_approval'
    tool_name TEXT DEFAULT '',
    question_text TEXT NOT NULL,
    options_json TEXT DEFAULT '[]',
    selected_indices_json TEXT DEFAULT '[]',
    resolved INTEGER NOT NULL DEFAULT 0,
    decision_json TEXT DEFAULT NULL,
    created_at INTEGER NOT NULL
);
