# 🚀 Telegram Harness Manager

> **Universal Multi-Harness Skill, Telegram Bot, Serverless Bridge & Mini App**  
> Control **Google Antigravity**, **Claude Code**, **OpenCode**, **DeepSeek Harness**, and **Aider** remotely from Telegram — with per-conversation **Forum Topics**, **Model / Thinking / Permission** controls, **Output-First notifications**, **Interactive Multi-Option Prompts**, and **Team Collaborator ACLs**.

🇬🇧 **English (Default)** | 🇮🇹 **[Leggi in Italiano (README_IT.md)](./README_IT.md)**

---

## ✨ Key Features

1. **🗂 1 Project → N Conversation Topics (Telegram Forum Mode)**
   - Just like in Antigravity or Claude Code, a single project can have **multiple parallel conversations**.
   - Each conversation is mapped to its own dedicated **Forum Topic** on Telegram (`📁 [project] 💬 Session Title`) so contexts never mix.

2. **⚙️ Per-Topic & Per-Prompt Execution Controls**
   - **🔌 Harness Engine**: Switch on the fly between `antigravity` (`agy`), `claude-code` (`claude`), `opencode`, `deepseek`, or `aider`.
   - **🤖 Model Selector**: Dynamically choose the model supported by the active harness.
   - **🧠 Thinking Level**: `⚡ None (Fast)`, `💡 Low`, `🧠 Medium`, `🔬 High`, `🌌 Max (UltraThink)`.
   - **🛡️ Agent Permissions**:
     - `📋 Plan (Read-Only)`: Agent can only read and propose plans without modifying files or running commands.
     - `🙋 Supervised (Ask)`: Agent asks for confirmation on Telegram (`PreToolUse` hook) before commands/writes.
     - `✏️ Auto-Edit`: Agent edits project files autonomously and asks only for shell commands.
     - `🚀 Full Auto (Turbo)`: Full autonomous execution until completion.
   - Inline prompt flags are also supported: e.g. `!high !plan Analyze the auth module`.

3. **⚡ Output-First Delivery + On-Demand Thinking & Git Diffs**
   - **Zero resource waste**: Does not spam your chat or drain battery with live token-by-token reasoning updates while you are away from your PC.
   - Sends **one clean message when the task finishes** (or pauses for a decision), embedding a native collapsible `<blockquote expandable>` with the steps/thinking summary and a **Telegram Mini App** view to inspect full *Thinking* logs and syntax-highlighted *Git Diffs* on demand.

4. **❓ Interactive Single-Select & Multi-Select Options**
   - When an harness invokes `ask_question` or proposes multiple options, the bot renders interactive inline buttons (including toggleable `☑️`/`⬜` checkboxes for multi-select questions and free-text reply support).

5. **👥 Team Collaboration, Guests & Telegram Forum Groups**
   - Share your local Harness with collaborators using granular per-project roles:
     - **`👁️ Viewer (Read-Only)`**: Can read final outputs, thinking steps, and Git diffs, but cannot launch prompts.
     - **`✋ Proposer (Manual Owner Approval)`**: Can submit prompts and options, which are held until the **Owner** taps **✅ Approve & Execute** (or downgrades to `Plan-Only`) on Telegram.
     - **`🚀 Executor (Direct Execution)`**: Trusted collaborator who can trigger prompts directly within the maximum permission ceiling set by the Owner.
   - **Group Forum Mode Security**:
     - Only a verified **Harness Owner** can add/link the bot to a Telegram Group.
     - Automatically verifies that **Forum Topics** (`is_forum`) are enabled and that the bot has **Administrator (`can_manage_topics`)** permissions.
     - Locks the default role for all group members to **`👁️ Viewer (Read-Only)`** by default so nobody can accidentally trigger prompts.

6. **🔒 Zero Bot Token Exposure for End Users (Dual-Mode Architecture)**
   - **Cloud Bridge Mode (`tgcloud`)**: The Bot Token lives exclusively inside **Telegram Serverless** (`tgcloud/`). User PCs authenticate to the bridge using only their personal `device_token` generated during `/start pair_<code>`. Every query is strictly isolated by `owner_user_id`.
   - **Self-Hosted Direct Mode**: Developers who prefer to use their own personal Telegram Bot can copy `.env.example` to `.env` and insert their own `TELEGRAM_BOT_TOKEN`.

---

## 🛠️ Quick Start & Installation

### 1. Build & Install the Skill + Plugin (Windows & Linux)

**Windows:**
```bat
install-plugin-windows.bat
```

**Linux / macOS:**
```bash
chmod +x install-plugin-linux.sh start-linux.sh
./install-plugin-linux.sh
```

This compiles the zero-dependency Rust binary (`telegram-harness-manager`) and installs the skill and hooks into:
- **Google Antigravity**: `~/.gemini/config/plugins/telegram-harness-manager` & `~/.gemini/config/skills/telegram-harness-manager`
- **Claude Code**: `~/.claude/skills/telegram-harness-manager`
- **OpenCode**: `~/.config/opencode/skills/telegram-harness-manager`
- **Workspace**: `.agents/skills/telegram-harness-manager/SKILL.md`

### 2. Configure `.env` (Only Placeholders in Git!)

Copy `.env.example` to `.env` (which is git-ignored):
```bash
cp .env.example .env
```
- **To use Cloud Bridge Mode**: Run `telegram-harness-manager pair` and click the deep-link in Telegram (no Bot Token required on your PC).
- **To use your own Bot (Self-Hosted)**: Create a bot via `@BotFather`, enable **Threaded Mode**, and set `TELEGRAM_BOT_NAME=@Your_Harness_Bot` and `TELEGRAM_BOT_TOKEN=YOUR_TELEGRAM_BOT_TOKEN_HERE` in `.env`.

### 3. Deploy to Telegram Serverless (`tgcloud`) — Optional for Bot Hosts

1. Copy `tgcloud.example.jsonc` to `tgcloud.jsonc` (git-ignored) and set your `appId`.
2. Run database migrations and push the V8 isolates + Mini App:
   ```bash
   npx tgcloud migrate
   npx tgcloud push
   ```

---

## 🌍 Internationalization (`i18n`) & Contributing Translations

Telegram Harness Manager automatically detects the user's language from Telegram's `user.language_code`:
- 🇮🇹 **Italian (`it`)**: `locales/it.json`
- 🇬🇧 **English (`en` - Default Fallback)**: `locales/en.json`

### Want to add a new language via Pull Request?
1. Copy `locales/en.json` to `locales/<lang_code>.json` (e.g., `locales/es.json`, `locales/de.json`, `locales/fr.json`).
2. Translate all string values while keeping `{param}` placeholders intact.
3. Register the locale in `tgcloud/i18n/index.js` (the Rust daemon automatically discovers `locales/<lang_code>.json` at runtime!).
4. Open a Pull Request!

---

## 📜 License (PEFL v1.0)

Distributed under the **Personal and Educational Free License (PEFL) v1.0**.  
Copyright © 2026 **Rubens Rainelli** ([rubens@rainelli.it](mailto:rubens@rainelli.it) | [GitHub](https://github.com/RubensRainelli)).

- **FREE** for Personal, Domestic, Educational, and Academic use.
- **PROHIBITED** in corporate, business, or commercial environments (including religious organizations) without written authorization and fee.
- See [LICENSE](./LICENSE), [LICENSE.it](./LICENSE.it), and [NOTICE.md](./NOTICE.md) for full terms.
