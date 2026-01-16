# CLASP Integration Test Suite

Comprehensive integration and load tests to verify the CLASP protocol implementation is real and functional.

## What This Tests

1. **OSC Integration** - Bidirectional communication with real OSC libraries (rosc)
2. **MIDI Integration** - MIDI message parsing/generation, virtual port detection
3. **Art-Net Integration** - DMX over Ethernet with artnet_protocol library
4. **CLASP-to-CLASP** - Inter-device communication, message encoding/decoding
5. **Security Model** - JWT capability tokens, scope enforcement, rate limits
6. **Load Testing** - Throughput, latency distribution, memory stability

## Running Tests

### Run All Tests
```bash
cargo run --bin run-all-tests
```

### Run Individual Test Suites
```bash
# OSC tests
cargo run --bin osc-integration

# MIDI tests
cargo run --bin midi-integration

# Art-Net tests
cargo run --bin artnet-integration

# CLASP protocol tests
cargo run --bin clasp-to-clasp

# Security tests
cargo run --bin security-tests

# Load tests
cargo run --bin load-tests
```

### Run Benchmarks
```bash
cargo bench
```

## Test Categories

### OSC Tests
- `OSC: Receive float from external sender`
- `OSC: Receive integer from external sender`
- `OSC: Receive string from external sender`
- `OSC: Receive blob from external sender`
- `OSC: Receive multiple arguments`
- `OSC: Send message to external receiver`
- `OSC: Handle bundle with timestamp`
- `OSC: Full encode/decode roundtrip`
- `OSC: High-rate message handling (1000 msgs)`

### MIDI Tests
- `MIDI: Parse Control Change messages`
- `MIDI: Parse Note On messages`
- `MIDI: Parse Note Off messages`
- `MIDI: Parse Program Change messages`
- `MIDI: Parse Pitch Bend messages`
- `MIDI: Parse SysEx messages`
- `MIDI: Parse Channel Pressure messages`
- `MIDI: Parse Poly Pressure messages`
- `MIDI: Generate valid MIDI messages`
- `MIDI: Check virtual port support`

### Art-Net Tests
- `Art-Net: Parse ArtDmx packets`
- `Art-Net: Generate valid ArtDmx packets`
- `Art-Net: Generate and parse ArtPoll`
- `Art-Net: Generate and parse ArtPollReply`
- `Art-Net: Handle multiple universes`
- `Art-Net: DMX value range 0-255`
- `Art-Net: Sequence number handling`
- `Art-Net: UDP roundtrip`

### CLASP Protocol Tests
- `CLASP: Message encoding (Hello)`
- `CLASP: Message decoding (Welcome)`
- `CLASP: All message types encode/decode`
- `CLASP: All value types`
- `CLASP: QoS levels in frames`
- `CLASP: Timestamp in frames`
- `CLASP: Address wildcard matching`
- `CLASP: Bundle message handling`
- `CLASP: Subscription pattern handling`
- `CLASP: State revision handling`

### Security Tests
- `Security: JWT token generation`
- `Security: JWT token validation`
- `Security: Read scope enforcement`
- `Security: Write scope enforcement`
- `Security: Address constraints (range)`
- `Security: Rate limit constraints`
- `Security: Expired token rejection`
- `Security: Invalid signature rejection`
- `Security: Wildcard scope patterns`
- `Security: Scope intersection`

### Load Tests
- `Load: Encoding throughput (10K msgs)` - Target: 50k+ msg/s
- `Load: Decoding throughput (10K msgs)` - Target: 50k+ msg/s
- `Load: Roundtrip throughput (5K msgs)` - Target: 20k+ msg/s
- `Load: Large payload (64KB)`
- `Load: Many small messages (50K)` - Target: 100k+ msg/s
- `Load: Concurrent encoding (4 threads)` - Target: 100k+ msg/s
- `Load: Memory stability (100K msgs)`
- `Load: Latency distribution (1K samples)` - Target: p99 < 1ms

## Dependencies

External libraries used to verify interoperability:
- `rosc` - Real OSC protocol implementation
- `midir` - Real MIDI I/O library
- `artnet_protocol` - Real Art-Net protocol implementation
- `jsonwebtoken` - JWT library for security testing
- `hdrhistogram` - High-precision latency measurement
- `criterion` - Benchmarking framework

## Output Example

```
   _____ _        _    ____  ____
  / ____| |      / \  / ___||  _ \
 | |    | |     / _ \ \___ \| |_) |
 | |____| |___ / ___ \ ___) |  __/
  \_____|_____/_/   \_\____/|_|

  Integration Test Suite v0.1.0
  Proving the protocol is REAL

============================================================
CLASP TEST SUITE RESULTS
============================================================

[PASS] OSC: Receive float from external sender (0.15ms)
[PASS] OSC: Send message to external receiver (0.23ms)
[PASS] MIDI: Parse Control Change messages (0.08ms)
[PASS] CLASP: All message types encode/decode (1.24ms)
[PASS] Security: JWT token validation (0.42ms)
[PASS] Load: Encoding throughput (10K msgs) (45.32ms)
...

------------------------------------------------------------
Total: 47 | Passed: 47 | Failed: 0
------------------------------------------------------------
```
