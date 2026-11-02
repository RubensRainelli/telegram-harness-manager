@echo off
REM Personal and Educational Free License (PEF License / PEFL) v1.0 - 2026-05-26
REM Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it | https://github.com/RubensRainelli/)

title Telegram Harness Manager - Daemon & Bot
cd /d "%~dp0"

if not exist "target\debug\telegram-harness-manager.exe" (
    cargo build
)

"target\debug\telegram-harness-manager.exe" serve
pause
