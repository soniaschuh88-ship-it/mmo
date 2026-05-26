#!/bin/sh
# start.sh — launch bifrost-server + Node.js gateway
#
# Works in two modes:
#   Docker: bifrost-server is at /usr/local/bin/bifrost-server (PATH)
#   Local:  bifrost-server is at ../target/debug/bifrost-server or ../target/release/bifrost-server

set -e

# ── Locate bifrost-server binary ──────────────────────────────────────────────
# Resolve the directory that contains this script, handling symlinks
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

find_binary() {
    # 1. Try PATH (Docker / installed case)
    if command -v bifrost-server >/dev/null 2>&1; then
        echo "bifrost-server"
        return
    fi
    # 2. Try release binary relative to this script (local dev, release build)
    if [ -f "$SCRIPT_DIR/../target/release/bifrost-server" ]; then
        echo "$SCRIPT_DIR/../target/release/bifrost-server"
        return
    fi
    # 3. Try debug binary (local dev, debug build)
    if [ -f "$SCRIPT_DIR/../target/debug/bifrost-server" ]; then
        echo "$SCRIPT_DIR/../target/debug/bifrost-server"
        return
    fi
    echo ""
}

BIFROST_BIN="$(find_binary)"

if [ -z "$BIFROST_BIN" ]; then
    echo "[start] ERROR: bifrost-server binary not found."
    echo "[start] Build it first:  cargo build -p bifrost-server"
    echo "[start] Or locally run:  ./dev.sh"
    exit 1
fi

# ── Start bifrost-server ───────────────────────────────────────────────────────
echo "[start] launching bifrost-server on :8081"
PORT=8081 "$BIFROST_BIN" &
RUST_PID=$!

# Wait up to 10s for bifrost-server to accept connections
echo "[start] waiting for bifrost-server to be ready..."
READY=0
for i in $(seq 1 20); do
    if wget -q -O/dev/null http://localhost:8081/health 2>/dev/null || \
       curl -sf http://localhost:8081/health >/dev/null 2>&1; then
        echo "[start] bifrost-server ready"
        READY=1
        break
    fi
    sleep 0.5
done

if [ "$READY" = "0" ]; then
    echo "[start] WARNING: bifrost-server did not respond — continuing anyway"
fi

# ── Start Node.js gateway ──────────────────────────────────────────────────────
# Gateway port comes from $PORT (Cloud Run sets this to 8080)
GATEWAY_PORT="${PORT:-8080}"
echo "[start] launching Node.js gateway on :${GATEWAY_PORT}"

# index.js must be found relative to this script's directory (app/)
cd "$SCRIPT_DIR"
PORT="${GATEWAY_PORT}" BIFROST_SERVER_URL=http://localhost:8081 node index.js
