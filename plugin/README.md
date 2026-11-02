# Telegram Harness Manager Plugin & Skill

Controlla **Google Antigravity**, **Claude Code**, **OpenCode** e **DeepSeek Harness** direttamente da Telegram tramite il bot configurato in `.env` (`@Your_Harness_Bot`).

## Funzionalità Principali
- **Multi-Conversation Forum Topics**: 1 Progetto → N Conversazioni (Topic separati su Telegram).
- **Controllo di Esecuzione**: Scegli **Harness**, **Modello**, **Livello di Pensiero** (`none`, `low`, `medium`, `high`, `max`) e **Permessi dell'Agente** (`plan`, `ask`, `auto_edit`, `auto`) prima o durante il lancio dei prompt.
- **Output-First + Thinking & Diff On-Demand**: Ricevi un unico messaggio pulito a fine task con `<blockquote expandable>` per i passaggi e accesso completo a Thinking e Git Diff nella Mini App.
- **Scelte Multiple Interattive**: Supporto per opzioni singole, checkbox multi-selezione e permessi tool (`PreToolUse`).
- **Collaborazione Multi-Utente & Gruppi**: Invita collaboratori con ruoli **Sola Lettura (`viewer`)**, **Modifica con Approvazione Owner (`proposer`)** o **Esecuzione Diretta (`executor`)**, oppure collega il bot a un Gruppo con Topic (verifica automatica Owner, Topic attivi e ruolo predefinito bloccato su Sola Lettura).
