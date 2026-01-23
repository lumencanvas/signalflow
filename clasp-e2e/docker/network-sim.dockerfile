# Network Simulation Docker Image
#
# Provides network impairment testing using Linux tc/netem.
# Requires --cap-add=NET_ADMIN to manipulate network settings.
#
# Build:
#   docker build -f network-sim.dockerfile -t clasp-network-sim ../..
#
# Run with network simulation:
#   docker run --cap-add=NET_ADMIN clasp-network-sim \
#     /usr/local/bin/simulate-network.sh 100 20 5

FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY . .

# Build test binaries
RUN cargo build --release -p clasp-e2e --bin network-simulation-tests
RUN cargo build --release -p clasp-e2e --bin chaos-tests

# Runtime image with network tools
FROM debian:bookworm-slim

# Install network simulation tools
RUN apt-get update && apt-get install -y \
    ca-certificates \
    iproute2 \
    iputils-ping \
    iptables \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/network-simulation-tests /usr/local/bin/
COPY --from=builder /app/target/release/chaos-tests /usr/local/bin/

# Copy network simulation script
COPY clasp-e2e/docker/simulate-network.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/simulate-network.sh

# Environment
ENV ROUTER_URL=ws://clasp-router:7330

ENTRYPOINT ["/bin/bash"]
