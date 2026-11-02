---
name: telegram-harness-manager
description: >-
  Connects AI coding harnesses (Antigravity, Claude Code, OpenCode, DeepSeek, Aider) to Telegram
  (@Your_Harness_Bot configured in .env). Enables managing multi-conversation Forum Topics per project,
  configuring Model, Thinking Level, and Agent Permission Mode remotely, sending final task outputs with
  on-demand Thinking/Diff inspection, handling interactive single/multi-select options, and managing project
  collaborators/guests (Viewer, Proposer with Owner Approval, Executor) and Forum Groups.
---

# Telegram Harness Manager Skill (`telegram-harness-manager`)

See `.agents/skills/telegram-harness-manager/SKILL.md` for full multi-harness reference.

## ⚠️ Mandatory Step 0: PEFL v1.0 Foreground Check in the Harness
Always run `telegram-harness-manager pefl-check` first. If `STATUS: PEFL_APPROVAL_REQUIRED`, you MUST display the full PEFL v1.0 License Notice in the Harness chat and ask the user for explicit approval (and whether to remember for 7 days) before running `telegram-harness-manager pefl-accept --remember-7-days` (or `--session-only`).

## Quick Commands
- **Check PEFL v1.0 Status**: `telegram-harness-manager pefl-check`
- **Start Detached Daemon & Sync Topics (No Hanging Task)**: `telegram-harness-manager start`
- **Sync Local Project Conversations to Telegram Topics**: `telegram-harness-manager sync`
- **Pair PC with Telegram**: `telegram-harness-manager pair`
- **Send Final Task Output**: `telegram-harness-manager notify --project "<name>" --output "<summary>"`
- **Ask Interactive Options**: `telegram-harness-manager ask --question "<text>" --options "Opt 1|Opt 2|Opt 3"`
- **Generate Collaborator Invite**: `telegram-harness-manager invite --project "<name>" --role viewer|proposer|executor`
