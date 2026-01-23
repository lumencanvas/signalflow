#!/bin/bash
# Network Simulation Script
#
# Adds network impairment using Linux tc/netem.
# Requires NET_ADMIN capability.
#
# Usage: simulate-network.sh <latency_ms> <jitter_ms> <loss_percent>
# Example: simulate-network.sh 50 10 1

set -e

LATENCY=${1:-0}
JITTER=${2:-0}
LOSS=${3:-0}
INTERFACE=${INTERFACE:-eth0}

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║              NETWORK SIMULATION CONFIGURATION                ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║ Interface: $INTERFACE"
echo "║ Latency:   ${LATENCY}ms (+/- ${JITTER}ms)"
echo "║ Packet Loss: ${LOSS}%"
echo "╚══════════════════════════════════════════════════════════════╝"

# Check if we have NET_ADMIN capability
if ! capsh --print 2>/dev/null | grep -q 'cap_net_admin'; then
    echo "Warning: NET_ADMIN capability may not be available"
    echo "Run container with: --cap-add=NET_ADMIN"
fi

# Clear any existing rules
tc qdisc del dev $INTERFACE root 2>/dev/null || true

# Apply network impairment if any values are non-zero
if [ "$LATENCY" -gt 0 ] || [ "$LOSS" != "0" ]; then
    CMD="tc qdisc add dev $INTERFACE root netem"

    if [ "$LATENCY" -gt 0 ]; then
        CMD="$CMD delay ${LATENCY}ms"
        if [ "$JITTER" -gt 0 ]; then
            CMD="$CMD ${JITTER}ms distribution normal"
        fi
    fi

    if [ "$LOSS" != "0" ] && [ "$LOSS" != "0.0" ]; then
        CMD="$CMD loss ${LOSS}%"
    fi

    echo "Executing: $CMD"
    eval $CMD

    echo ""
    echo "Network simulation active. Current qdisc:"
    tc qdisc show dev $INTERFACE
else
    echo "No impairment configured (all values are 0)"
fi

echo ""
echo "Ready for testing."
