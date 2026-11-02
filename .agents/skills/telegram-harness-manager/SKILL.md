---
name: telegram-harness-manager
description: >-
  Connects AI coding harnesses (Antigravity, Claude Code, OpenCode, DeepSeek, Aider) to Telegram
  (@Your_Harness_Bot or a configured bot in .env). Enables managing multi-conversation Forum Topics per project,
  configuring Model, Thinking Level, and Agent Permission Mode remotely, sending final task outputs with
  on-demand Thinking/Diff inspection, handling interactive single/multi-select options, and managing project
  collaborators/guests (Viewer, Proposer with Owner Approval, Executor) and Forum Groups.
---

# Telegram Harness Manager Skill (`telegram-harness-manager`)

Use this skill whenever the user wants to:
- Connect or pair their machine/project with Telegram (`@Your_Harness_Bot` configured in `.env`);
- Manage project conversations remotely via Telegram **Forum Topics** (1 Project → Multiple Conversation Topics);
- Configure **Model**, **Thinking Level** (`none`, `low`, `medium`, `high`, `max`), and **Agent Permissions** (`plan`, `ask`, `auto_edit`, `auto`) before or during prompt execution;
- Receive **Output-First notifications** when a task finishes (without wasting resources on live thinking spam, while keeping full *Thinking* and *Git Diffs* inspectable on demand);
- Present **interactive single-select or multi-select choices** (`ask_question` / tool approvals) on Telegram;
- Invite **collaborators/guests** to a project with granular roles (`viewer` read-only, `proposer` requiring manual Owner approval, or `executor` direct execution), or link a Telegram Forum Group safely.

---

## 🛠️ CLI & Daemon Commands (`telegram-harness-manager`)

The `telegram-harness-manager` binary (installed in `~/.gemini/antigravity/bin` or workspace `target/release`) provides zero-dependency integration across Windows and Linux:

### 1. Pair This Machine with Telegram (Zero Bot Token Exposure for End Users)
```bash
telegram-harness-manager pair --accept-pefl
```
Generates a cryptographic `device_token` stored locally in `.thm/device.json` and prints a one-time Telegram deep-link:
`https://t.me/<YOUR_BOT_USERNAME>?start=pair_<code>`

### 2. Start the Local Daemon & Telegram Bridge
```bash
telegram-harness-manager serve --accept-pefl
```
- In **Cloud Mode (`THM_MODE=cloud`)**: Polls the Telegram Serverless (`tgcloud`) bridge using only the user's `device_token` (the Bot Token never leaves Telegram Serverless).
- In **Direct Mode (`THM_MODE=direct`)**: Uses `TELEGRAM_BOT_TOKEN` from `.env` (never committed to Git) to serve your configured bot directly.
- Also serves the local companion API & Mini App preview on port `8430` (`http://127.0.0.1:8430`).

### 3. Send Final Output to the Conversation's Telegram Topic
When you complete a task and the user asked to be notified on Telegram (or automatically via the `Stop` hook):
```bash
telegram-harness-manager notify \
  --project "telegram-harness-manager" \
  --conversation "<conversation-id>" \
  --harness "antigravity" \
  --output "Completed implementation and verified all tests." \
  --thinking "1. Built tgcloud V8 endpoints\n2. Added multi-topic & ACL support"
```

### 4. Ask Interactive Multiple-Choice Questions on Telegram
When you need the user to choose among several options (single-select or multi-select) remotely:
```bash
telegram-harness-manager ask \
  --conversation "<conversation-id>" \
  --question "Which deployment mode do you want to run?" \
  --options "Telegram Serverless (Recommended)|Direct Local Bot|Both" \
  --multi false
```

### 5. Manage Collaborators / Guests on a Project
```bash
telegram-harness-manager invite \
  --project "telegram-harness-manager" \
  --role "proposer"
```
Roles supported:
- `viewer`: Read-only access to outputs, thinking logs, and diffs.
- `proposer`: Can propose prompts and options, which are paused until the Project Owner taps **✅ Approve & Execute** on Telegram.
- `executor`: Direct prompt execution within the maximum permission mode set by the Owner.

---

## 🔌 Supported Harnesses

| Harness | CLI Binary | Headless / Remote Dispatch |
| :--- | :--- | :--- |
| **Antigravity** | `agy` | `agy chat --reuse-window -m "<prompt>"` + Hooks (`PreToolUse`, `Stop`) |
| **Claude Code** | `claude` | `claude -p "<prompt>" --model <model> --permission-mode <mode>` |
| **OpenCode** | `opencode` | `opencode run "<prompt>" --model <model>` |
| **DeepSeek Harness** | `deepseek` / `aider` | `aider --model deepseek/<model> --message "<prompt>"` |
| **Custom CLI** | Custom | Configured in `.thm/harnesses.json` |
