# 🚀 Telegram Harness Manager (Guida in Italiano)

> **Skill Universale Multi-Harness, Bot Telegram, Bridge Serverless & Mini App**  
> Gestisci **Google Antigravity**, **Claude Code**, **OpenCode**, **DeepSeek Harness** e **Aider** da remoto su Telegram — con **Forum Topic multi-conversazione** per ogni progetto, controllo di **Modello / Thinking / Permessi**, notifiche **Output-First**, **Scelte Multiple Interattive** e **Collaboratori con Permessi Granulari**.

🇮🇹 **Italiano** | 🇬🇧 **[Read in English (README.md)](./README.md)**

---

## ✨ Caratteristiche Principali

1. **🗂 1 Progetto → N Conversazioni / Topic (Modalità Forum)**
   - Proprio come in Antigravity, per ogni progetto puoi aprire **più conversazioni parallele**.
   - Ogni conversazione corrisponde a un **Forum Topic** dedicato su Telegram (`📁 [progetto] 💬 Titolo Sessione`).

2. **⚙️ Controllo di Modello, Livello di Pensiero e Permessi Agente**
   - **🔌 Harness**: Cambia al volo tra `antigravity` (`agy`), `claude-code` (`claude`), `opencode`, `deepseek` o `aider`.
   - **🤖 Modello**: Seleziona dinamicamente il modello supportato dall'harness attivo.
   - **🧠 Livello di Thinking**: `⚡ Nessuno (Rapido)`, `💡 Basso`, `🧠 Medio`, `🔬 Alto`, `🌌 Massimo (UltraThink)`.
   - **🛡️ Permessi dell'Agente**:
     - `📋 Plan (Sola Lettura)`: L'agente può solo analizzare e proporre piani senza modificare file o lanciare comandi.
     - `🙋 Supervisionato (Chiedi)`: Chiede conferma interattiva su Telegram prima di comandi o scritture.
     - `✏️ Auto-Edit File`: Modifica i file in autonomia e chiede conferma solo per comandi terminale.
     - `🚀 Full Auto (Turbo)`: Esecuzione autonoma completa fino a fine task.

3. **⚡ Notifica a Fine Task (Output-First) + Thinking & Diff On-Demand**
   - Nessuno spreco di risorse o notifiche continue mentre l'agente ragiona.
   - Ricevi **un unico messaggio con l'Output finale** quando il lavoro è concluso, completo di blocco nativo espandibile `<blockquote expandable>` con i passaggi eseguiti e accesso completo a *Thinking* e *Git Diff* tramite la **Telegram Mini App**.

4. **❓ Opzioni Multiple Interattive (Single & Multi-Select)**
   - Supporta sia le approvazioni dei tool (`PreToolUse`) sia le domande a scelta singola o multipla (`ask_question`) con pulsanti inline, checkbox interattive `☑️`/`⬜` e risposta testuale libera.

5. **👥 Invitati, Collaboratori e Gruppi Telegram con Topic**
   - L'Owner dell'Harness può invitare collaboratori sui singoli progetti assegnando uno dei 3 ruoli:
     - **`👁️ Sola Lettura (Viewer)`**: Vede output, thinking e diff ma non può lanciare prompt.
     - **`✋ Modifica con Approvazione Manuale (Proposer)`**: Può inviare prompt che restano in pausa finché l'**Owner** non preme **✅ Approva ed Esegui** su Telegram.
     - **`🚀 Esecuzione Diretta (Executor)`**: Esegue prompt direttamente entro il limite massimo di permessi fissato dall'Owner.
   - **Sicurezza Gruppi con Topic**:
     - Solo l'**Owner verificato dell'Harness** può aggiungere e configurare il bot in un gruppo.
     - Verifica automatica che i **Topic (Forum)** siano attivi nel gruppo e che il bot sia **Amministratore con permesso `Gestire i Topic`**.
     - Imposta automaticamente tutti i membri del gruppo su **`👁️ Sola Lettura (Viewer)`** di default per evitare che qualcuno faccia danni.

---

## 🛠️ Installazione Rapida

### 1. Compila e Installa la Skill + Plugin
Su **Windows**:
```bat
install-plugin-windows.bat
```
Su **Linux / macOS**:
```bash
chmod +x install-plugin-linux.sh start-linux.sh
./install-plugin-linux.sh
```

### 2. Configurazione `.env` (Zero Segreti su GitHub)
Copia `.env.example` in `.env` (ignorato da Git):
- Chi vuole usare il proprio bot può inserire i propri dati al posto dei placeholder `YOUR_TELEGRAM_BOT_TOKEN_HERE` e `@Your_Harness_Bot`.
- Chi usa la modalità Cloud Bridge esegue `telegram-harness-manager pair` senza bisogno del Bot Token sul proprio PC.

---

## 📜 Licenza (PEFL v1.0)

Distribuito sotto licenza **Personal and Educational Free License (PEFL) v1.0**.  
Copyright © 2026 **Rubens Rainelli** ([rubens@rainelli.it](mailto:rubens@rainelli.it) | [GitHub](https://github.com/RubensRainelli)).  
Uso gratuito per scopi personali, domestici, didattici e accademici. Uso aziendale, professionale o commerciale vietato senza autorizzazione scritta e pagamento della fee. Consulta [LICENSE.it](./LICENSE.it) e [LICENSE](./LICENSE).
