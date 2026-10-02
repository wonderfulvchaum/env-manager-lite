#!/bin/sh
# Env Manager Lite - 入口脚本
# 用法: docker run --rm -e DB_HOST=... -e API_KEY=... image [python|node|bash|go|rust]

MODE="${1:-python}"

case $MODE in
  python|py)
    echo "🐍 Running Python validator..."
    python3 /app/examples/validate.py
    ;;
  node|js|javascript)
    echo "🟨 Running Node.js validator..."
    node /app/examples/validate.js
    ;;
  bash|sh|shell)
    echo "🐚 Running Bash validator..."
    sh /app/examples/validate.sh
    ;;
  go|golang)
    echo "🐹 Running Go validator..."
    go run /app/examples/validate.go
    ;;
  rust|rs)
    echo "🦀 Running Rust validator..."
    cargo script /app/examples/validate.rs 2>/dev/null || rustc /app/examples/validate.rs -o /tmp/validate_rust && /tmp/validate_rust
    ;;
  help|*)
    cat <<'EOF'
Env Manager Lite - Multi-language Environment Validator

Usage:
  docker run --rm -e DB_HOST=... -e API_KEY=... <image> [mode]

Modes:
  python    Python validator (default)
  node      Node.js validator
  bash      Bash validator
  go        Go validator
  rust      Rust validator

Required env vars (from .env-required):
  DB_HOST, API_KEY
EOF
    exit 0
    ;;
esac
