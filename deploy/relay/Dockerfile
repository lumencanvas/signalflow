# CLASP Relay Server Dockerfile
# 
# This is a STANDALONE deployment that uses published crates from crates.io.
# No monorepo dependencies required.
#
# Build:
#   docker build -t clasp-relay .
#
# Run:
#   docker run -p 7330:7330 clasp-relay
#
# For development (using local crates), build from repo root with:
#   docker build -f deploy/relay/Dockerfile.dev -t clasp-relay-dev .

FROM rust:1.75-slim-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the relay source (standalone)
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

# Build the relay binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/clasp-relay /usr/local/bin/clasp-relay

# Create non-root user
RUN useradd -r -s /bin/false clasp
USER clasp

# WebSocket port
EXPOSE 7330

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD timeout 1 bash -c 'cat < /dev/null > /dev/tcp/localhost/7330' || exit 1

ENTRYPOINT ["clasp-relay"]
CMD ["--port", "7330", "--name", "relay.clasp.to"]
