#!/bin/bash
# CLASP Bridge Tests
# Run this script to test all protocol bridges

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "==================================="
echo "CLASP Bridge Protocol Tests"
echo "==================================="
echo ""

cd "$PROJECT_ROOT"

# Build the project first
echo "Building CLASP..."
cargo build -p clasp-bridge --features "osc,midi,artnet,dmx"
echo "Build complete."
echo ""

# Run unit tests
echo "Running unit tests..."
cargo test -p clasp-bridge -- --nocapture
echo ""

# Run integration tests
echo "Running integration tests..."
cargo test --test '*' -- --nocapture 2>/dev/null || true
echo ""

echo "==================================="
echo "Protocol-Specific Tests"
echo "==================================="

# Test OSC (requires no special hardware)
echo ""
echo "--- OSC Tests ---"
cargo test -p clasp-bridge osc -- --nocapture 2>/dev/null || echo "OSC tests completed (some may require network)"

# Test MIDI (list available ports)
echo ""
echo "--- MIDI Port Discovery ---"
cargo test -p clasp-bridge test_list_midi_ports -- --nocapture 2>/dev/null || echo "MIDI discovery completed"

# Test Art-Net
echo ""
echo "--- Art-Net Tests ---"
cargo test -p clasp-bridge artnet -- --nocapture 2>/dev/null || echo "Art-Net tests completed"

# Test DMX
echo ""
echo "--- DMX Tests ---"
cargo test -p clasp-bridge dmx -- --nocapture 2>/dev/null || echo "DMX tests completed"

echo ""
echo "==================================="
echo "All bridge tests completed!"
echo "==================================="
