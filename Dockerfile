# ── Stage 1: Build Rust bifrost-server + bifrost-wasm bundle ──────────────────
FROM rust:1-slim AS rust-builder

# Install C linker, curl (for wasm-pack installer), and SSL headers.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

# Add the WebAssembly target (needed by wasm-pack / bifrost-wasm).
RUN rustup target add wasm32-unknown-unknown

# Install wasm-pack — used to build bifrost/wasm into a browser JS+WASM bundle.
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /build

# Copy workspace manifest + all crate sources.
# nexus/ and nova/ are required because bifrost-server and bifrost-wasm
# depend on nexus-voxel-kernel and nova-* transitively.
COPY Cargo.toml Cargo.lock ./
COPY bifrost/ bifrost/
COPY nexus/   nexus/
COPY nova/    nova/

# Build the native release binary.
RUN cargo build --release -p bifrost-server

# Build the WASM bundle used by game.html.
# Output goes to /wasm-out so we can COPY it into the Node.js stage cleanly.
RUN wasm-pack build bifrost/wasm \
      --target web \
      --release \
      --out-dir /wasm-out \
      --out-name bifrost_wasm

# ── Stage 2: Node.js gateway + Rust binary + WASM bundle ──────────────────────
FROM node:20-slim

# Run as non-root (better compatibility with Docker rootless / Cloud Run).
RUN groupadd -r appuser && useradd -r -g appuser -m -d /app appuser

WORKDIR /app

# Install Node.js dependencies.
COPY app/package*.json ./
RUN npm install --omit=dev

# Copy gateway source (includes game.html, admin.html, index.js, …).
COPY app/ ./

# ── WASM bundle ── served as /pkg/bifrost_wasm/*.{js,wasm}
# game.html loads this via:
#   import init from '/pkg/bifrost_wasm/bifrost_wasm.js';
COPY --from=rust-builder /wasm-out/ ./pkg/bifrost_wasm/

# Copy compiled Rust binary.
COPY --from=rust-builder /build/target/release/bifrost-server /usr/local/bin/bifrost-server

# Ensure non-root can read/execute everything it needs.
RUN chmod +x /app/start.sh /usr/local/bin/bifrost-server && \
    chown -R appuser:appuser /app /usr/local/bin/bifrost-server

USER appuser

# bifrost-server runs internally on 8081.
# Node.js gateway is exposed on $PORT (Cloud Run default: 8080).
ENV PORT=8080
ENV BIFROST_SERVER_URL=http://localhost:8081

EXPOSE 8080

CMD ["/app/start.sh"]
