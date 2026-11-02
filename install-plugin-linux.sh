#!/usr/bin/env bash
# Personal and Educational Free License (PEF License / PEFL) v1.0 - 2026-05-26
# Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it | https://github.com/RubensRainelli/)
# Personal and educational free use permitted. Corporate use prohibited without written authorization and fee.

set -e
cd "$(dirname "$0")"

echo "============================================================================"
echo "  Installing Multi-Harness Skill & Plugin: Telegram Harness Manager"
echo "  Supported: Antigravity, Claude Code, OpenCode, DeepSeek Harness, Aider"
echo "  Author: Rubens Rainelli (PEFL v1.0)"
echo "============================================================================"

echo "[1/4] Building telegram-harness-manager..."
cargo build

BIN_DIR="$HOME/.local/bin"
AGY_BIN_DIR="$HOME/.gemini/antigravity/bin"
AGY_PLUGIN_DIR="$HOME/.gemini/config/plugins/telegram-harness-manager"
AGY_SKILL_DIR="$HOME/.gemini/config/skills/telegram-harness-manager"
CLAUDE_SKILL_DIR="$HOME/.claude/skills/telegram-harness-manager"
OPENCODE_SKILL_DIR="$HOME/.config/opencode/skills/telegram-harness-manager"

mkdir -p "$BIN_DIR" "$AGY_BIN_DIR" "$AGY_PLUGIN_DIR/assets" "$AGY_SKILL_DIR" "$CLAUDE_SKILL_DIR" "$OPENCODE_SKILL_DIR"

echo "[2/4] Copying binary..."
cp -f target/debug/telegram-harness-manager "$BIN_DIR/telegram-harness-manager"
cp -f target/debug/telegram-harness-manager "$AGY_BIN_DIR/telegram-harness-manager"
cp -f target/debug/telegram-harness-manager "$AGY_PLUGIN_DIR/telegram-harness-manager"
chmod +x "$BIN_DIR/telegram-harness-manager" "$AGY_BIN_DIR/telegram-harness-manager"

echo "[3/4] Installing plugin & hooks..."
cp -f plugin/plugin.json "$AGY_PLUGIN_DIR/plugin.json"
cp -f plugin/hooks.json "$AGY_PLUGIN_DIR/hooks.json"
cp -f plugin/README.md "$AGY_PLUGIN_DIR/README.md"
cp -f plugin/assets/logo.svg "$AGY_PLUGIN_DIR/assets/logo.svg"

echo "[4/4] Installing SKILL.md across Antigravity, Claude Code, and OpenCode..."
cp -f .agents/skills/telegram-harness-manager/SKILL.md "$AGY_SKILL_DIR/SKILL.md"
cp -f .agents/skills/telegram-harness-manager/SKILL.md "$CLAUDE_SKILL_DIR/SKILL.md"
cp -f .agents/skills/telegram-harness-manager/SKILL.md "$OPENCODE_SKILL_DIR/SKILL.md"

echo "[OK] Telegram Harness Manager installed successfully!"
