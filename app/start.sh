#!/bin/sh
# start.sh — launch bifrost-server (Rust) + gateway (Node.js) together

set -e

# Start bifrost-server in background on internal port 8081
echo "[start] launching bifrost-server on :8081"
PORT=8081 bifrost-server &
RUST_PID=$!

# Wait for bifrost-server to accept connections (max 10s)
echo "[start] waiting for bifrost-server to be ready..."
for i in $(seq 1 20); do
    if wget -q -O/dev/null http://localhost:8081/health 2>/dev/null; then
        echo "[start] bifrost-server ready"
        break
    fi
    sleep 0.5
done

# Start Node.js gateway in foreground (Cloud Run expects this process)
echo "[start] launching Node.js gateway on :${PORT}"
PORT=${PORT:-8080} BIFROST_SERVER_URL=http://localhost:8081 node index.js
