@echo off
REM Personal and Educational Free License (PEF License / PEFL) v1.0 - 2026-05-26
REM Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it | https://github.com/RubensRainelli/)
REM Personal and educational free use permitted. Corporate use prohibited without written authorization and fee.

title Installazione Skill ^& Plugin: Telegram Harness Manager
cd /d "%~dp0"

echo ============================================================================
echo   Installazione Multi-Harness Skill ^& Plugin: Telegram Harness Manager
echo   Supporto: Antigravity, Claude Code, OpenCode, DeepSeek Harness, Aider
echo   Autore: Rubens Rainelli (PEFL v1.0)
echo ============================================================================
echo.

echo [1/4] Compilazione del binario telegram-harness-manager.exe...
cargo build
if errorlevel 1 (
    echo [ERRORE] Compilazione fallita. Verifica che Rust/Cargo sia installato.
    pause
    exit /b 1
)

set "BIN_DIR=%USERPROFILE%\.gemini\antigravity\bin"
set "WIN_APPS_DIR=%USERPROFILE%\AppData\Local\Microsoft\WindowsApps"
set "AGY_PLUGIN_DIR=%USERPROFILE%\.gemini\config\plugins\telegram-harness-manager"
set "AGY_SKILL_DIR=%USERPROFILE%\.gemini\config\skills\telegram-harness-manager"
set "CLAUDE_SKILL_DIR=%USERPROFILE%\.claude\skills\telegram-harness-manager"
set "OPENCODE_SKILL_DIR=%USERPROFILE%\.config\opencode\skills\telegram-harness-manager"

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%AGY_PLUGIN_DIR%\assets" mkdir "%AGY_PLUGIN_DIR%\assets"
if not exist "%AGY_SKILL_DIR%" mkdir "%AGY_SKILL_DIR%"
if not exist "%CLAUDE_SKILL_DIR%" mkdir "%CLAUDE_SKILL_DIR%"
if not exist "%OPENCODE_SKILL_DIR%" mkdir "%OPENCODE_SKILL_DIR%"

echo [2/4] Copia di telegram-harness-manager.exe nel PATH utente e nel plugin...
copy /Y "target\debug\telegram-harness-manager.exe" "%BIN_DIR%\telegram-harness-manager.exe" >nul
copy /Y "target\debug\telegram-harness-manager.exe" "%AGY_PLUGIN_DIR%\telegram-harness-manager.exe" >nul
if exist "%WIN_APPS_DIR%" (
    copy /Y "target\debug\telegram-harness-manager.exe" "%WIN_APPS_DIR%\telegram-harness-manager.exe" >nul
)

echo [3/4] Installazione Plugin ^& Hooks per Google Antigravity...
copy /Y "plugin\plugin.json" "%AGY_PLUGIN_DIR%\plugin.json" >nul
copy /Y "plugin\hooks.json" "%AGY_PLUGIN_DIR%\hooks.json" >nul
copy /Y "plugin\README.md" "%AGY_PLUGIN_DIR%\README.md" >nul
copy /Y "plugin\assets\logo.svg" "%AGY_PLUGIN_DIR%\assets\logo.svg" >nul

echo [4/4] Installazione SKILL.md per Antigravity, Claude Code e OpenCode...
copy /Y ".agents\skills\telegram-harness-manager\SKILL.md" "%AGY_SKILL_DIR%\SKILL.md" >nul
copy /Y ".agents\skills\telegram-harness-manager\SKILL.md" "%CLAUDE_SKILL_DIR%\SKILL.md" >nul
copy /Y ".agents\skills\telegram-harness-manager\SKILL.md" "%OPENCODE_SKILL_DIR%\SKILL.md" >nul

echo.
echo [OK] Skill e Plugin "Telegram Harness Manager" installati con successo in:
echo      - %AGY_PLUGIN_DIR%
echo      - %AGY_SKILL_DIR%
echo      - %CLAUDE_SKILL_DIR%
echo      - %OPENCODE_SKILL_DIR%
echo      - %BIN_DIR%\telegram-harness-manager.exe
echo.
echo Per avviare il demone / bot esegui: start-windows.bat
if "%1"=="--no-pause" exit /b 0
pause
