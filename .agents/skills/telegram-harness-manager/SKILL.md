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

## ⚠️ MANDATORY STEP 0: Foreground PEFL v1.0 License Check in the Harness

Under **Clause 3 of the Personal and Educational Free License (PEFL) v1.0**, before executing ANY command of `telegram-harness-manager`, the AI Coding Harness **MUST** verify whether the license has been accepted within the last 7 days by running:

```bash
telegram-harness-manager pefl-check
```

If the output indicates `STATUS: PEFL_APPROVAL_REQUIRED` (exit code `42`):
1. **You MUST display the full PEFL v1.0 License box below directly in the foreground of the Harness chat** so the user can read it clearly:
   ```text
   ============================================================
        Personal and Educational Free License (PEFL) v1.0
   ============================================================
   Software: Telegram Harness Manager
   Copyright (c) 2026 Rubens Rainelli
   Official Specification: https://github.com/RubensRainelli/pefl-license-spec
   Author Contact: rubens@rainelli.it | Tel: (+39) 342 811 5882

   TERMINI D'USO / TERMS OF USE:
   * GRATUITO per uso Personale, Domestico, Didattico ed Educativo
     (FREE for Personal, Domestic, Educational & Academic use).
   * SEVERAMENTE VIETATO l'uso Aziendale, Professionale o Commerciale
     senza autorizzazione scritta e pagamento del corrispettivo all'Autore.
   * Esclude luoghi di culto e organizzazioni religiose (uso aziendale).
   ============================================================
   ```
2. **You MUST ask the user explicitly** (using `ask_question` or a direct question in chat) to choose one of:
   - `(Recommended) Accetto la licenza PEFL v1.0 (Ricorda per 7 giorni)`
   - `Accetto la licenza PEFL v1.0 solo per questa sessione`
   - `Rifiuto la licenza PEFL v1.0`
3. **NEVER execute `telegram-harness-manager pefl-accept` without first showing the license in the Harness chat and receiving the user's explicit consent.**
4. Once the user accepts, run:
   ```bash
   telegram-harness-manager pefl-accept --remember-7-days
   # or, if they chose session-only:
   telegram-harness-manager pefl-accept --session-only
   ```

---

## 🛠️ CLI & Daemon Commands (`telegram-harness-manager`)

After PEFL v1.0 compliance is verified (`STATUS: PEFL_ACCEPTED`), use these commands:

### 1. Pair This Machine with Telegram (Zero Bot Token Exposure for End Users)
```bash
telegram-harness-manager pair
```
Generates a cryptographic `device_token` stored locally in `.thm/device.json` and prints a one-time Telegram deep-link:
`https://t.me/<YOUR_BOT_USERNAME>?start=pair_<code>`

### 2. Start the Local Daemon & Sync Project Topics (Detached — Zero Hanging Task in Antigravity!)
```bash
telegram-harness-manager start
```
- **IMPORTANT FOR HARNESS AGENTS**: Always run `telegram-harness-manager start` synchronously (`IsDaemon: false` in `run_command`). It automatically spawns the background daemon as a detached OS process (`CREATE_NO_WINDOW | DETACHED_PROCESS`), syncs all local project conversations into Telegram **Forum Topics**, and exits immediately in `< 1s` without leaving any background task hanging in the Antigravity UI!
- To re-sync local conversations/projects into Telegram Forum Topics at any time:
  ```bash
  telegram-harness-manager sync
  ```
- To check daemon status or stop it:
  ```bash
  telegram-harness-manager status
  telegram-harness-manager stop
  ```

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
