#!/usr/bin/env bash
# dev.sh — run the full stack locally (Rust + Node.js)
#
# Usage:
#   ./dev.sh              # build debug binary + start both services
#   ./dev.sh --release    # build release binary (slower build, faster runtime)
#   ./dev.sh --port 3000  # Node gateway on a different port (default: 3000)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GATEWAY_PORT="${GATEWAY_PORT:-3000}"
BIFROST_PORT="${BIFROST_PORT:-8081}"
BUILD_PROFILE="debug"

# ── Parse flags ────────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --release) BUILD_PROFILE="release" ;;
        --port)    shift; GATEWAY_PORT="$1" ;;
    esac
done

# ── Build Rust binary ──────────────────────────────────────────────────────────
BINARY_PATH="$REPO_ROOT/target/$BUILD_PROFILE/bifrost-server"

if [ ! -f "$BINARY_PATH" ]; then
    echo "[dev] building bifrost-server ($BUILD_PROFILE)..."
    if [ "$BUILD_PROFILE" = "release" ]; then
        cargo build --release -p bifrost-server
    else
        cargo build -p bifrost-server
    fi
fi

echo "[dev] bifrost-server: $BINARY_PATH"

# ── Node.js deps ───────────────────────────────────────────────────────────────
APP_DIR="$REPO_ROOT/app"
if [ ! -d "$APP_DIR/node_modules" ]; then
    echo "[dev] installing Node.js dependencies..."
    (cd "$APP_DIR" && npm install)
fi

# ── Start bifrost-server ───────────────────────────────────────────────────────
echo "[dev] starting bifrost-server on :$BIFROST_PORT"
PORT="$BIFROST_PORT" "$BINARY_PATH" &
RUST_PID=$!
trap 'echo "[dev] stopping..."; kill "$RUST_PID" 2>/dev/null; exit 0' INT TERM EXIT

# Wait for bifrost-server to be ready
echo "[dev] waiting for bifrost-server..."
for i in $(seq 1 20); do
    if curl -sf "http://localhost:$BIFROST_PORT/health" >/dev/null 2>&1; then
        echo "[dev] bifrost-server ready"
        break
    fi
    sleep 0.5
done

# ── Start Node.js gateway ──────────────────────────────────────────────────────
echo "[dev] starting gateway on http://localhost:$GATEWAY_PORT"
echo "[dev] bifrost API at  http://localhost:$BIFROST_PORT"
echo ""
PORT="$GATEWAY_PORT" \
BIFROST_SERVER_URL="http://localhost:$BIFROST_PORT" \
node "$APP_DIR/index.js"
