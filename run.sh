#!/usr/bin/env bash
set -e

MODE="${1:-gui}"

case "$MODE" in
  tui)
    cargo run -p starkbot-app
    ;;
  gui)
    # Always rebuild frontend to pick up changes
    FRONTEND_DIR="crates/starkbot-tauri/frontend"
    echo "Building frontend..."
    (cd "$FRONTEND_DIR" && npm install --silent && npm run build)
    cargo run -p starkbot-tauri
    ;;
  *)
    echo "Usage: ./run.sh [tui|gui]"
    echo "  gui  - Desktop GUI (Tauri, default)"
    echo "  tui  - Terminal UI"
    exit 1
    ;;
esac
