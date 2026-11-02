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

## Quick Commands
- **Pair PC with Telegram**: `telegram-harness-manager pair --accept-pefl`
- **Start Daemon / Bot Bridge**: `telegram-harness-manager serve --accept-pefl`
- **Send Final Task Output**: `telegram-harness-manager notify --project "<name>" --output "<summary>"`
- **Ask Interactive Options**: `telegram-harness-manager ask --question "<text>" --options "Opt 1|Opt 2|Opt 3"`
- **Generate Collaborator Invite**: `telegram-harness-manager invite --project "<name>" --role viewer|proposer|executor`
