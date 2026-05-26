# ── Stage 1: Build Rust bifrost-server ────────────────────────────────────────
FROM rust:1.82-slim AS rust-builder

# Install C linker (needed for linking)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifest + all crate sources
COPY Cargo.toml Cargo.lock ./
COPY bifrost/ bifrost/

# Build release binary
RUN cargo build --release -p bifrost-server

# ── Stage 2: Node.js gateway + Rust binary ────────────────────────────────────
FROM node:20-slim

# Run as non-root (better compatibility with Docker rootless)
RUN groupadd -r appuser && useradd -r -g appuser -m -d /app appuser

WORKDIR /app


# Install Node.js dependencies
COPY app/package*.json ./
RUN npm install --omit=dev

# Copy gateway source
COPY app/ ./

# Copy compiled Rust binary
COPY --from=rust-builder /build/target/release/bifrost-server /usr/local/bin/bifrost-server

# Ensure non-root can read/execute everything it needs
RUN chmod +x /app/start.sh /usr/local/bin/bifrost-server && \
    chown -R appuser:appuser /app /usr/local/bin/bifrost-server

USER appuser


# bifrost-server runs internally on 8081
# Node.js gateway is exposed on $PORT (Cloud Run default: 8080)
ENV PORT=8080
ENV BIFROST_SERVER_URL=http://localhost:8081

EXPOSE 8080

CMD ["/app/start.sh"]
