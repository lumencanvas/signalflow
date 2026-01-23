# CLASP Docker Test Infrastructure

Docker configurations for running CLASP tests at scale.

## Quick Start

```bash
# Build all images
docker-compose -f docker-compose.load-test.yml build

# Run load tests (100 clients, 60 seconds)
docker-compose -f docker-compose.load-test.yml up

# Run chaos tests
docker-compose -f docker-compose.chaos-test.yml up
```

## Available Images

### Router (`router.dockerfile`)
Minimal CLASP router image for testing and deployment.

```bash
docker build -f router.dockerfile -t clasp-router ../..
docker run -p 7330:7330 clasp-router
```

### Load Generator (`load-generator.dockerfile`)
Load testing tools with configurable parameters.

```bash
docker build -f load-generator.dockerfile -t clasp-load-generator ../..
docker run -e ROUTER_URL=ws://host.docker.internal:7330 clasp-load-generator
```

Environment variables:
- `ROUTER_URL` - WebSocket URL of router (default: `ws://clasp-router:7330`)
- `NUM_CLIENTS` - Number of concurrent clients (default: 100)
- `MESSAGES_PER_CLIENT` - Messages per client (default: 1000)
- `DURATION_SECS` - Test duration in seconds (default: 60)

### Network Simulator (`network-sim.dockerfile`)
Network impairment testing with Linux tc/netem.

```bash
docker build -f network-sim.dockerfile -t clasp-network-sim ../..
docker run --cap-add=NET_ADMIN clasp-network-sim \
  /usr/local/bin/simulate-network.sh 100 20 5
```

Network simulation parameters:
- First arg: Latency in milliseconds
- Second arg: Jitter in milliseconds
- Third arg: Packet loss percentage

## Compose Configurations

### Load Test (`docker-compose.load-test.yml`)
Full load testing setup with router and generator.

```bash
# Default configuration
docker-compose -f docker-compose.load-test.yml up

# Custom configuration
NUM_CLIENTS=1000 DURATION_SECS=300 \
  docker-compose -f docker-compose.load-test.yml up
```

### Chaos Test (`docker-compose.chaos-test.yml`)
Chaos engineering tests including network simulation.

```bash
docker-compose -f docker-compose.chaos-test.yml up
```

## Manual Server Testing

For testing on a dedicated server:

```bash
# 1. Start router
./target/release/clasp-router --bind 0.0.0.0:7330 &

# 2. Run load tests from another machine
ROUTER_URL=ws://your-server:7330 cargo run --release -p clasp-e2e --bin load-tests

# 3. Monitor resources
htop  # CPU/memory
ss -s  # Socket stats
```

## Network Simulation on Linux (without Docker)

Requires root access:

```bash
# Add 50ms latency with 10ms jitter and 1% loss
sudo tc qdisc add dev lo root netem delay 50ms 10ms loss 1%

# Run tests
cargo test -p clasp-e2e --bin network-simulation-tests

# Remove impairment
sudo tc qdisc del dev lo root
```

## Network Simulation on macOS (without Docker)

Requires sudo:

```bash
# Enable packet filter
sudo pfctl -e

# Create dummynet pipe with 50ms delay
sudo dnctl pipe 1 config delay 50ms
echo "dummynet in on lo0 pipe 1" | sudo pfctl -f -

# Run tests
cargo run -p clasp-e2e --bin network-simulation-tests

# Disable
sudo pfctl -d
```
