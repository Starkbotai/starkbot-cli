#!/usr/bin/env bash
set -e

MODE="${1:-tui}"

case "$MODE" in
  tui)
    cargo run -p starkbot-app
    ;;
  gui)
    # Build frontend if dist doesn't exist
    FRONTEND_DIR="crates/starkbot-tauri/frontend"
    if [ ! -d "$FRONTEND_DIR/dist" ]; then
      echo "Building frontend..."
      cd "$FRONTEND_DIR" && npm install && npm run build && cd - > /dev/null
    fi
    cargo run -p starkbot-tauri
    ;;
  *)
    echo "Usage: ./run.sh [tui|gui]"
    echo "  tui  - Terminal UI (default)"
    echo "  gui  - Desktop GUI (Tauri)"
    exit 1
    ;;
esac
