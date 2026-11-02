#!/usr/bin/env bash
# Personal and Educational Free License (PEF License / PEFL) v1.0 - 2026-05-26
# Copyright (c) 2026 Rubens Rainelli (rubens@rainelli.it | https://github.com/RubensRainelli/)

set -e
cd "$(dirname "$0")"

if [ ! -f "target/debug/telegram-harness-manager" ]; then
    cargo build
fi

./target/debug/telegram-harness-manager serve
