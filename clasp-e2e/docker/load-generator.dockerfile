# CLASP Load Generator Docker Image
#
# Runs load tests against a CLASP router.
# Configurable via environment variables.
#
# Build:
#   docker build -f load-generator.dockerfile -t clasp-load-generator ../..
#
# Run:
#   docker run -e ROUTER_URL=ws://router:7330 clasp-load-generator

FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Build load test binaries
RUN cargo build --release -p clasp-e2e --bin load-tests
RUN cargo build --release -p clasp-e2e --bin sustained-load-benchmarks
RUN cargo build --release -p clasp-e2e --bin soak-tests

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/load-tests /usr/local/bin/
COPY --from=builder /app/target/release/sustained-load-benchmarks /usr/local/bin/
COPY --from=builder /app/target/release/soak-tests /usr/local/bin/

# Environment variables for configuration
ENV ROUTER_URL=ws://clasp-router:7330
ENV NUM_CLIENTS=100
ENV MESSAGES_PER_CLIENT=1000
ENV DURATION_SECS=60

# Default: run load tests
CMD ["load-tests"]
