# CLASP Relay Server Dockerfile
# Multi-stage build for minimal image size
#
# DEPLOYMENT OPTIONS:
#
# 1. DigitalOcean App Platform (WebSocket only):
#    - Use default build: docker build -t clasp-relay .
#    - Only WebSocket works (UDP not supported)
#    - Handles TLS termination for you
#
# 2. Droplet/VPS (Full transport support):
#    - Build with QUIC: docker build --build-arg FEATURES=full -t clasp-relay .
#    - Supports WebSocket + QUIC
#    - You handle TLS (Let's Encrypt, etc.)

ARG FEATURES=websocket

FROM rust:1.75-slim-bookworm as builder

ARG FEATURES

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY tools ./tools
COPY bindings ./bindings

# Build the clasp-router binary with specified features
# - default (websocket): Works on DO App Platform
# - full (websocket + quic): Requires UDP, use on Droplet/VPS
RUN cargo build --release -p clasp-router-server --features "${FEATURES}"

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/clasp-router /usr/local/bin/clasp-router

# Create non-root user
RUN useradd -r -s /bin/false clasp
USER clasp

# Ports:
# 7330 - WebSocket (TCP) - works everywhere
# 7331 - QUIC (UDP) - requires Droplet/VPS, NOT supported on App Platform
EXPOSE 7330
EXPOSE 7331/udp

# Health check - WebSocket port
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD timeout 1 bash -c 'cat < /dev/null > /dev/tcp/localhost/7330' || exit 1

# Run the router
# Override CMD for different configurations:
#   WebSocket only: --listen 0.0.0.0:7330 --transport websocket
#   QUIC only: --listen 0.0.0.0:7331 --transport quic
ENTRYPOINT ["clasp-router"]
CMD ["--listen", "0.0.0.0:7330", "--transport", "websocket", "--name", "relay.clasp.to"]
