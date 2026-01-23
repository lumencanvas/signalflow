# CLASP Router Docker Image
#
# Builds a minimal image for running the CLASP router in containers.
# Used for load testing, CI, and production deployments.
#
# Build:
#   docker build -f router.dockerfile -t clasp-router ../..
#
# Run:
#   docker run -p 7330:7330 clasp-router

FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Build release binary
RUN cargo build --release -p clasp-router

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/clasp-router /usr/local/bin/

# Default port
EXPOSE 7330

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD timeout 2 bash -c '</dev/tcp/localhost/7330' || exit 1

# Run router
CMD ["clasp-router", "--bind", "0.0.0.0:7330"]
