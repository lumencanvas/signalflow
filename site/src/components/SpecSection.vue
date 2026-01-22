<script setup>
import { ref } from 'vue'
import CodeBlock from './CodeBlock.vue'

const activeSection = ref('quickstart')

const specSections = ref([
  { id: 'quickstart', title: '0. Quick Start', open: true },
  { id: 'overview', title: '1. What is CLASP?', open: false },
  { id: 'handshake', title: '2. Connection Flow', open: false },
  { id: 'addresses', title: '3. Addresses & Wildcards', open: false },
  { id: 'signals', title: '4. Signal Types', open: false },
  { id: 'messages', title: '5. Message Reference', open: false },
  { id: 'frame', title: '6. Wire Format', open: false },
  { id: 'types', title: '7. Data Types', open: false },
  { id: 'bridges', title: '8. Protocol Bridges', open: false },
  { id: 'timing', title: '9. Clock Sync', open: false },
  { id: 'discovery', title: '10. Discovery', open: false },
  { id: 'security', title: '11. Security', open: false },
  { id: 'benchmarks', title: '12. Benchmarks', open: false }
])

function toggleSection(section) {
  section.open = !section.open
  if (section.open) {
    activeSection.value = section.id
  }
}

function scrollToSection(id) {
  const section = specSections.value.find(s => s.id === id)
  if (section) {
    section.open = true
    activeSection.value = id
  }
  document.getElementById(`spec-${id}`)?.scrollIntoView({ behavior: 'smooth' })
}

// Quick start - minimal working example
const quickstartJS = `import { Clasp } from '@clasp-to/core';

// 1. Connect to a CLASP router
const clasp = new Clasp('ws://localhost:7330');
await clasp.connect();

// 2. Listen for changes (pattern matching with wildcards)
clasp.on('/lights/*/brightness', (value, address) => {
  console.log(\`\${address} changed to \${value}\`);
});

// 3. Set a value (automatically synced to all subscribers)
clasp.set('/lights/living-room/brightness', 0.75);

// 4. Emit a one-shot event
clasp.emit('/scene/activate', { name: 'movie-mode' });`

// Connection handshake
const handshakeFlow = `Client                           Server
  |                                |
  |-- WebSocket Connect ---------->|  (ws://host:7330, subprotocol: clasp.v3)
  |                                |
  |-- HELLO ---------------------->|  { version: 3, name: "My App", features: [...] }
  |                                |
  |<--------- WELCOME -------------|  { session: "abc123", time: 1704067200000000 }
  |                                |
  |-- SUBSCRIBE ------------------>|  { pattern: "/lights/**" }
  |                                |
  |<--------- SNAPSHOT ------------|  { params: [{ address: "/lights/1", value: 0.5 }] }
  |                                |
  |-- SET ------------------------>|  { address: "/lights/1", value: 0.8 }
  |                                |
  |<--------- SET (broadcast) -----|  (all subscribers receive this)`

const helloMsg = `// HELLO - sent by client after WebSocket connects
{
  "type": "HELLO",
  "version": 3,
  "name": "My Controller App",
  "features": ["param", "event", "stream"],
  "token": "cpsk_7kX9mP2nQ4rT6vW8xZ0aB3cD5eF1gH"  // optional CPSK token
}`

const welcomeMsg = `// WELCOME - server response with session info
{
  "type": "WELCOME",
  "version": 3,
  "session": "sess_a1b2c3",      // unique session ID
  "name": "CLASP Router",
  "time": 1704067200000000,      // server time in microseconds
  "features": ["param", "event", "stream", "gesture", "timeline"]
}`

// Address patterns
const addressExamples = `// Addresses are hierarchical paths (like URLs or OSC addresses)
/lights/kitchen/brightness       // specific light
/lights/kitchen/color            // another property
/audio/master/volume             // audio mixer
/midi/launchpad/note/60          // MIDI note

// Wildcards for subscriptions:
/lights/*/brightness             // * matches one segment
/lights/**                       // ** matches any depth
/midi/*/cc/*                     // multiple wildcards OK`

// Signal types with clear examples
const signalExamplesParam = `// PARAM: Stateful values that persist and sync
// Use for: faders, toggles, settings, anything with "current state"

clasp.set('/mixer/channel/1/volume', 0.75);

// Server tracks: current value, revision number, last writer
// All subscribers get updates when value changes
// Late joiners receive current state via SNAPSHOT`

const signalExamplesEvent = `// EVENT: One-shot triggers that don't persist
// Use for: button presses, cue triggers, notifications

clasp.emit('/cue/fire', { cueId: 'intro', fadeTime: 2.0 });

// No state stored - if you miss it, it's gone
// Good for: triggers, commands, notifications`

const signalExamplesStream = `// STREAM: High-frequency data (30-60+ Hz)
// Use for: sensor data, audio levels, motion tracking

clasp.stream('/accelerometer/x', 0.342);

// Uses "fire and forget" delivery (no ACK)
// Supports rate limiting and epsilon filtering:
clasp.subscribe('/sensors/**', callback, {
  maxRate: 30,    // max 30 updates/sec
  epsilon: 0.01   // ignore changes smaller than 1%
});`

// Wire format
const frameCode = `┌──────────────────────────────────────────────────────────────────┐
│ CLASP Frame Format (4-12 byte header + payload)                  │
├──────────────────────────────────────────────────────────────────┤
│ Byte 0:    Magic byte 0x53 ('S' for Signal)                      │
│ Byte 1:    Flags                                                 │
│            ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐    │
│            │  7  │  6  │  5  │  4  │  3  │  2  │  1  │  0  │    │
│            │  QoS (2b) │ TS  │ Enc │ Cmp │ Version (3b)  │    │
│            └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘    │
│ Bytes 2-3: Payload length (uint16 big-endian, max 65535)         │
│ [Optional] Bytes 4-11: Timestamp (uint64 microseconds)           │
│ Payload:   v3 compact binary (version=1) or MessagePack (v=0)    │
└──────────────────────────────────────────────────────────────────┘

Version bits (0-2):
  000 = v2 legacy (MessagePack with named keys)
  001 = v3 compact (positional binary, 54% smaller)

QoS Values (bits 6-7):
  00 = Fire    - Best effort, no ACK (streams)
  01 = Confirm - At-least-once, server sends ACK (params, events)
  10 = Commit  - Exactly-once, ordered (bundles, timelines)`

// Message reference
const setMsg = `// SET - write a param value
{
  "type": "SET",
  "address": "/lights/1/brightness",
  "value": 0.75,
  "revision": 42      // optional: for conflict detection
}`

const subscribeMsg = `// SUBSCRIBE - register for updates matching a pattern
{
  "type": "SUBSCRIBE",
  "id": 1,                          // client-assigned ID for unsubscribe
  "pattern": "/lights/**",          // wildcard pattern
  "types": ["param", "event"],      // optional: filter by signal type
  "options": {
    "maxRate": 30,                  // optional: rate limit
    "epsilon": 0.01                 // optional: ignore tiny changes
  }
}`

const publishMsg = `// PUBLISH - send events, streams, or gestures
{
  "type": "PUBLISH",
  "address": "/cue/fire",
  "signal": "event",                // "event" | "stream" | "gesture"
  "payload": { "cueId": "intro" },  // for events
  "value": 0.75,                    // for streams
  "timestamp": 1704067200000000
}`

const bundleMsg = `// BUNDLE - atomic group of messages, optionally scheduled
{
  "type": "BUNDLE",
  "timestamp": 1704067300000000,    // execute at this time (optional)
  "messages": [
    { "type": "SET", "address": "/light/1/intensity", "value": 1.0 },
    { "type": "SET", "address": "/light/2/intensity", "value": 0.5 },
    { "type": "PUBLISH", "address": "/cue/fire", "signal": "event", "payload": {} }
  ]
}`

// Data types
const dataTypesCode = `// v3 compact binary value types:
Type     Code   Encoding            Example Use
───────────────────────────────────────────────────────────
null     0x00   1 byte              No value
bool     0x01   1 byte (false)      Toggles
bool     0x02   1 byte (true)       Toggles
f64      0x07   9 bytes             Most numeric values
string   0x08   2+N bytes (len+utf8) Labels, names
bytes    0x09   4+N bytes           Binary data
array    0x0A   4+N×X bytes         Lists
map      0x0B   4+N×X bytes         Objects

// In practice, most values are just numbers or simple types:
clasp.set('/volume', 0.8);                    // f64
clasp.set('/mute', true);                     // bool
clasp.set('/label', 'Main Mix');              // string
clasp.set('/color', { r: 255, g: 128, b: 0}); // map`

// Bridges
const bridgesCode = `Protocol bridges translate between CLASP and legacy protocols.
Each bridge is a CLASP client that speaks both languages.

┌────────────┬───────────────────────────────────────────────────┐
│ Protocol   │ CLASP Address Mapping                             │
├────────────┼───────────────────────────────────────────────────┤
│ MIDI CC    │ /midi/{device}/cc/{channel}/{number}  (Param)    │
│ MIDI Note  │ /midi/{device}/note/{channel}/{note}  (Event)    │
│ OSC        │ Preserves OSC path as CLASP address              │
│ Art-Net    │ /artnet/{universe}/{channel}          (Param)    │
│ DMX        │ /dmx/{universe}/{channel}             (Param)    │
│ MQTT       │ /mqtt/{topic}                         (varies)   │
│ HTTP       │ REST API at /api/v1/...                          │
└────────────┴───────────────────────────────────────────────────┘

Example: MIDI CC 7 on channel 1 from "launchpad"
         → /midi/launchpad/cc/1/7 (value 0.0-1.0)`

// Timing
const timingCode = `// Clock sync uses NTP-style exchange (SYNC messages)
Client                           Server
  |                                |
  |-- SYNC { t1 } ---------------->|  t1 = client send time
  |                                |
  |<----- SYNC { t1, t2, t3 } -----|  t2 = server receive
  |        t4 = client receive     |  t3 = server send

// Calculate offset
roundTrip = (t4 - t1) - (t3 - t2)
offset = ((t2 - t1) + (t3 - t4)) / 2

// Use for scheduled bundles:
clasp.bundle([...], { at: clasp.time() + 100000 }); // 100ms from now

// All timestamps in CLASP are microseconds since Unix epoch`

// Discovery
const discoveryCode = `// 1. mDNS (recommended for LAN)
Service type: _clasp._tcp.local
TXT record: { version: "3", name: "My App", ws: "7330" }

// 2. UDP broadcast fallback (port 7331)
Client broadcasts: HELLO
Server responds:   ANNOUNCE with connection info

// 3. Manual / QR code
For browsers (can't do mDNS/UDP) or WAN connections,
provide the WebSocket URL directly: wss://example.com:7330`

// Security
const securityCode = `## Security Modes

### 1. OPEN (default for local dev)
   - No encryption, no auth
   - Use only on trusted networks

### 2. TRANSPORT ENCRYPTED
   - WSS (TLS 1.3) via reverse proxy
   - QUIC with built-in TLS 1.3
   - Wire-level encryption without application auth

### 3. TOKEN AUTHENTICATED
   Capability Pre-Shared Keys (CPSK) with scoped permissions:

   # Generate token via CLI
   clasp token create --scopes "read:/**,write:/lights/**"
   # Output: cpsk_7kX9mP2nQ4rT6vW8xZ0aB3cD5eF1gH

   # Use in client
   const client = await Clasp.builder('wss://venue.example.com')
     .token('cpsk_7kX9mP2nQ4rT6vW8xZ0aB3cD5eF1gH')
     .connect();

   Scope patterns:
   - read:/**        - Subscribe to all
   - write:/lights/* - Control lights namespace
   - admin:/**       - Full access

   Optional: External PASETO/JWT tokens for federated auth`

// Message catalog
const messages = [
  { name: 'HELLO', code: '0x01', desc: 'Client introduction with name, version, features' },
  { name: 'WELCOME', code: '0x02', desc: 'Server response with session ID and server time' },
  { name: 'SUBSCRIBE', code: '0x10', desc: 'Register for updates matching a pattern' },
  { name: 'UNSUBSCRIBE', code: '0x11', desc: 'Remove a subscription by ID' },
  { name: 'SET', code: '0x21', desc: 'Write a param value' },
  { name: 'GET', code: '0x22', desc: 'Request current value of an address' },
  { name: 'SNAPSHOT', code: '0x23', desc: 'Bulk response with multiple param values' },
  { name: 'PUBLISH', code: '0x20', desc: 'Send event, stream, or gesture' },
  { name: 'BUNDLE', code: '0x30', desc: 'Atomic group of messages, optionally scheduled' },
  { name: 'SYNC', code: '0x40', desc: 'Clock synchronization (NTP-style)' },
  { name: 'PING/PONG', code: '0x41/42', desc: 'Keep-alive and latency measurement' },
  { name: 'ACK', code: '0x50', desc: 'Delivery confirmation' },
  { name: 'ERROR', code: '0x51', desc: 'Error response with code and message' }
]

// Signal type definitions
const signalTypes = [
  { name: 'Param', qos: 'Confirm', persist: 'Yes', desc: 'Stateful values (faders, settings). Changes sync to all subscribers. Revision-tracked.' },
  { name: 'Event', qos: 'Confirm', persist: 'No', desc: 'One-shot triggers (button press, cue fire). No state stored.' },
  { name: 'Stream', qos: 'Fire', persist: 'No', desc: 'High-rate data (30-60+ Hz). Rate-limited. Lossy but fast.' },
  { name: 'Gesture', qos: 'Fire', persist: 'No', desc: 'Phased input (start/move/end). For touch, pen, mouse drag.' },
  { name: 'Timeline', qos: 'Commit', persist: 'Yes', desc: 'Automation lanes. Time-indexed keyframes for playback.' }
]

// Benchmark data (from cargo test --package clasp-core --release)
const benchmarks = {
  encoding: [
    { proto: 'MQTT', rate: '11.4M', winner: true },
    { proto: 'CLASP v3', rate: '8M', winner: false },
    { proto: 'OSC', rate: '4.5M', winner: false }
  ],
  decoding: [
    { proto: 'MQTT', rate: '11.4M', winner: true },
    { proto: 'CLASP v3', rate: '11M', winner: false },
    { proto: 'OSC', rate: '5.7M', winner: false }
  ],
  size: [
    { proto: 'MQTT', bytes: 19, winner: true },
    { proto: 'OSC', bytes: 24, winner: false },
    { proto: 'CLASP v3', bytes: 31, winner: false }
  ],
  features: [
    { feature: 'State synchronization', clasp: true, osc: false, mqtt: false },
    { feature: 'Late-joiner support', clasp: true, osc: false, mqtt: true },
    { feature: 'Typed signals (Param/Event/Stream)', clasp: true, osc: false, mqtt: false },
    { feature: 'QoS levels', clasp: '3', osc: '0', mqtt: '3' },
    { feature: 'Token security with scopes', clasp: true, osc: false, mqtt: true },
    { feature: 'Multi-protocol bridging', clasp: true, osc: false, mqtt: false },
    { feature: 'Clock sync', clasp: true, osc: true, mqtt: false },
    { feature: 'Wildcard subscriptions', clasp: true, osc: false, mqtt: true }
  ]
}
</script>

<template>
  <section class="section" id="spec">
    <h2>FULL SPEC (CLASP v3)</h2>

    <div class="spec-wrap">
      <aside class="spec-toc">
        <div class="toc-title">CONTENTS</div>
        <a
          v-for="section in specSections"
          :key="section.id"
          :class="{ active: activeSection === section.id }"
          @click="scrollToSection(section.id)"
        >
          {{ section.title }}
        </a>
      </aside>

      <article class="spec-body">
        <!-- 0. Quick Start -->
        <section
          :id="`spec-quickstart`"
          class="spec-section"
          :class="{ open: specSections[0].open }"
        >
          <h3 @click="toggleSection(specSections[0])">0. Quick Start</h3>
          <div class="spec-content">
            <p>Get a working CLASP connection in 10 lines of JavaScript:</p>
            <CodeBlock :code="quickstartJS" language="javascript" />
            <p style="margin-top: 1rem;">That's it. The client handles the handshake, state sync, and reconnection automatically.</p>
          </div>
        </section>

        <!-- 1. What is CLASP -->
        <section
          :id="`spec-overview`"
          class="spec-section"
          :class="{ open: specSections[1].open }"
        >
          <h3 @click="toggleSection(specSections[1])">1. What is CLASP?</h3>
          <div class="spec-content">
            <p>CLASP is a <b>universal protocol bridge</b> for creative applications. It connects everything: MIDI controllers, OSC apps, DMX lights, Art-Net fixtures, MQTT sensors, and WebSocket interfaces through a single unified address space.</p>

            <p style="margin-top: 1rem;">Under the hood, it's a <b>pub/sub protocol</b> (like MQTT) optimized for real-time media. But the killer feature is that it <b>bridges all your existing gear</b>: your TouchOSC tablet can control your DMX lights while your MIDI controller adjusts your VJ software, all through CLASP.</p>

            <p style="margin-top: 1rem;"><b>Core concepts:</b></p>
            <ul>
              <li><b>Router:</b> Central server that routes messages between clients (like an MQTT broker)</li>
              <li><b>Addresses:</b> Hierarchical paths like <code>/lights/kitchen/brightness</code></li>
              <li><b>Signals:</b> Five types: Param (stateful), Event (one-shot), Stream (high-rate), Gesture (phased input), Timeline (automation)</li>
              <li><b>Wildcards:</b> Subscribe to patterns: <code>/lights/*</code> or <code>/lights/**</code></li>
            </ul>

            <p style="margin-top: 1rem;"><b>Why not just use OSC/MIDI/MQTT directly?</b></p>
            <ul>
              <li><b>vs OSC:</b> CLASP has state. Late-joining clients get current values, not just future changes.</li>
              <li><b>vs MIDI:</b> CLASP has meaningful addresses (not channel/CC numbers) and works over networks.</li>
              <li><b>vs MQTT:</b> CLASP has typed signals (param vs event), built-in clock sync, and sub-ms latency.</li>
            </ul>

            <p style="margin-top: 1rem;">You don't have to choose. CLASP bridges them all. Keep using your existing gear and software.</p>
          </div>
        </section>

        <!-- 2. Connection Flow -->
        <section
          :id="`spec-handshake`"
          class="spec-section"
          :class="{ open: specSections[2].open }"
        >
          <h3 @click="toggleSection(specSections[2])">2. Connection Flow</h3>
          <div class="spec-content">
            <p>The complete sequence from connect to receiving data:</p>
            <CodeBlock :code="handshakeFlow" language="plaintext" />

            <p style="margin-top: 1rem;"><b>HELLO message (client sends first):</b></p>
            <CodeBlock :code="helloMsg" language="json" />

            <p style="margin-top: 1rem;"><b>WELCOME message (server response):</b></p>
            <CodeBlock :code="welcomeMsg" language="json" />

            <p style="margin-top: 1rem;"><b>Key points:</b></p>
            <ul>
              <li>WebSocket subprotocol is <code>clasp.v3</code></li>
              <li>Default port is <code>7330</code></li>
              <li>Server time is in <b>microseconds</b> (not milliseconds)</li>
              <li>After WELCOME, you can immediately SUBSCRIBE and start sending</li>
            </ul>
          </div>
        </section>

        <!-- 3. Addresses & Wildcards -->
        <section
          :id="`spec-addresses`"
          class="spec-section"
          :class="{ open: specSections[3].open }"
        >
          <h3 @click="toggleSection(specSections[3])">3. Addresses & Wildcards</h3>
          <div class="spec-content">
            <p>Addresses are slash-separated paths, like URLs or file paths:</p>
            <CodeBlock :code="addressExamples" language="plaintext" />

            <p style="margin-top: 1rem;"><b>Wildcard rules:</b></p>
            <ul>
              <li><code>*</code> matches exactly one path segment (like <code>[^/]+</code> regex)</li>
              <li><code>**</code> matches zero or more segments (like <code>.*</code> regex)</li>
              <li>Wildcards work in SUBSCRIBE patterns only, not in SET/PUBLISH addresses</li>
            </ul>

            <p style="margin-top: 1rem;"><b>Examples:</b></p>
            <ul>
              <li><code>/lights/*/brightness</code> matches <code>/lights/kitchen/brightness</code> and <code>/lights/bedroom/brightness</code></li>
              <li><code>/lights/**</code> matches <code>/lights/kitchen</code>, <code>/lights/kitchen/brightness</code>, and <code>/lights/kitchen/color/r</code></li>
              <li><code>/midi/*/cc/*/*</code> matches any MIDI CC from any device</li>
            </ul>
          </div>
        </section>

        <!-- 4. Signal Types -->
        <section
          :id="`spec-signals`"
          class="spec-section"
          :class="{ open: specSections[4].open }"
        >
          <h3 @click="toggleSection(specSections[4])">4. Signal Types</h3>
          <div class="spec-content">
            <p>CLASP has five signal types, each optimized for different use cases:</p>

            <div class="table">
              <div class="row head">
                <div>Type</div>
                <div>QoS</div>
                <div>State</div>
                <div>Use Case</div>
              </div>
              <div class="row" v-for="sig in signalTypes" :key="sig.name">
                <div><b>{{ sig.name }}</b></div>
                <div>{{ sig.qos }}</div>
                <div>{{ sig.persist }}</div>
                <div>{{ sig.desc }}</div>
              </div>
            </div>

            <p style="margin-top: 1.5rem;"><b>Param (most common):</b></p>
            <CodeBlock :code="signalExamplesParam" language="javascript" />

            <p style="margin-top: 1rem;"><b>Event:</b></p>
            <CodeBlock :code="signalExamplesEvent" language="javascript" />

            <p style="margin-top: 1rem;"><b>Stream:</b></p>
            <CodeBlock :code="signalExamplesStream" language="javascript" />
          </div>
        </section>

        <!-- 5. Message Reference -->
        <section
          :id="`spec-messages`"
          class="spec-section"
          :class="{ open: specSections[5].open }"
        >
          <h3 @click="toggleSection(specSections[5])">5. Message Reference</h3>
          <div class="spec-content">
            <p>All messages start with a <code>type</code> byte (v3 compact binary) or <code>type</code> field (v2 MessagePack):</p>

            <div class="table">
              <div class="row head">
                <div>Message</div>
                <div>Code</div>
                <div>Description</div>
              </div>
              <div class="row" v-for="msg in messages" :key="msg.name">
                <div><b>{{ msg.name }}</b></div>
                <div><code>{{ msg.code }}</code></div>
                <div>{{ msg.desc }}</div>
              </div>
            </div>

            <p style="margin-top: 1.5rem;"><b>SET message:</b></p>
            <CodeBlock :code="setMsg" language="json" />

            <p style="margin-top: 1rem;"><b>SUBSCRIBE message:</b></p>
            <CodeBlock :code="subscribeMsg" language="json" />

            <p style="margin-top: 1rem;"><b>PUBLISH message:</b></p>
            <CodeBlock :code="publishMsg" language="json" />

            <p style="margin-top: 1rem;"><b>BUNDLE message:</b></p>
            <CodeBlock :code="bundleMsg" language="json" />
          </div>
        </section>

        <!-- 6. Wire Format -->
        <section
          :id="`spec-frame`"
          class="spec-section"
          :class="{ open: specSections[6].open }"
        >
          <h3 @click="toggleSection(specSections[6])">6. Wire Format</h3>
          <div class="spec-content">
            <p>Each CLASP message is wrapped in a binary frame:</p>
            <CodeBlock :code="frameCode" language="plaintext" />

            <p style="margin-top: 1rem;"><b>Implementation notes:</b></p>
            <ul>
              <li>Minimum frame size: 4 bytes (header) + 1 byte (payload) = 5 bytes</li>
              <li>Maximum payload: 65535 bytes (larger data should be chunked)</li>
              <li>Timestamps are optional but recommended for bundles and streams</li>
              <li>Compression (if enabled) uses LZ4 or zstd</li>
            </ul>
          </div>
        </section>

        <!-- 7. Data Types -->
        <section
          :id="`spec-types`"
          class="spec-section"
          :class="{ open: specSections[7].open }"
        >
          <h3 @click="toggleSection(specSections[7])">7. Data Types</h3>
          <div class="spec-content">
            <p>Values use v3 compact binary encoding (54% smaller than MessagePack). Most of the time you'll just use numbers, strings, and objects:</p>
            <CodeBlock :code="dataTypesCode" language="plaintext" />
          </div>
        </section>

        <!-- 8. Protocol Bridges -->
        <section
          :id="`spec-bridges`"
          class="spec-section"
          :class="{ open: specSections[8].open }"
        >
          <h3 @click="toggleSection(specSections[8])">8. Protocol Bridges</h3>
          <div class="spec-content">
            <CodeBlock :code="bridgesCode" language="plaintext" />

            <p style="margin-top: 1rem;"><b>Implemented bridges:</b></p>
            <ul>
              <li><b>OSC:</b> Bidirectional, bundle support, timestamp preservation</li>
              <li><b>MIDI:</b> CC, notes, program change, pitchbend (via midir)</li>
              <li><b>Art-Net:</b> Multiple universes, polling, delta detection</li>
              <li><b>DMX:</b> ENTTEC Pro/Open, FTDI adapters</li>
              <li><b>MQTT:</b> v3.1.1 and v5, TLS support</li>
              <li><b>WebSocket:</b> Generic JSON bridge</li>
              <li><b>HTTP:</b> REST API for request/response patterns</li>
            </ul>
          </div>
        </section>

        <!-- 9. Clock Sync -->
        <section
          :id="`spec-timing`"
          class="spec-section"
          :class="{ open: specSections[9].open }"
        >
          <h3 @click="toggleSection(specSections[9])">9. Clock Sync</h3>
          <div class="spec-content">
            <p>CLASP uses NTP-style synchronization for scheduled bundles:</p>
            <CodeBlock :code="timingCode" language="plaintext" />

            <p style="margin-top: 1rem;"><b>When to use timestamps:</b></p>
            <ul>
              <li>Scheduled bundles (execute at a specific time)</li>
              <li>Stream data (for interpolation/buffering)</li>
              <li>Gesture events (for latency compensation)</li>
            </ul>
            <p>Target sync accuracy: ±1ms on LAN, ±5ms on WiFi</p>
          </div>
        </section>

        <!-- 10. Discovery -->
        <section
          :id="`spec-discovery`"
          class="spec-section"
          :class="{ open: specSections[10].open }"
        >
          <h3 @click="toggleSection(specSections[10])">10. Discovery</h3>
          <div class="spec-content">
            <p>Three ways to find CLASP routers:</p>
            <CodeBlock :code="discoveryCode" language="plaintext" />

            <p style="margin-top: 1rem;"><b>Browser limitations:</b> Browsers can't do mDNS or UDP. Use manual URL entry, QR codes, or a companion app that discovers and shares the URL.</p>
          </div>
        </section>

        <!-- 11. Security -->
        <section
          :id="`spec-security`"
          class="spec-section"
          :class="{ open: specSections[11].open }"
        >
          <h3 @click="toggleSection(specSections[11])">11. Security</h3>
          <div class="spec-content">
            <CodeBlock :code="securityCode" language="plaintext" />

            <p style="margin-top: 1rem;"><b>Recommendations:</b></p>
            <ul>
              <li>Local dev: Open mode is fine</li>
              <li>Production LAN: Use WSS (encrypted)</li>
              <li>Public internet: Use WSS + CPSK tokens</li>
            </ul>
          </div>
        </section>

        <!-- 12. Benchmarks -->
        <section
          :id="`spec-benchmarks`"
          class="spec-section"
          :class="{ open: specSections[12].open }"
        >
          <h3 @click="toggleSection(specSections[12])">12. Benchmarks</h3>
          <div class="spec-content">
            <p class="bench-intro"><b>Why CLASP when MQTT and OSC are faster?</b> Because raw serialization speed isn't everything. CLASP trades some encoding speed for features that matter in real-time creative applications:</p>

            <div class="feature-table">
              <div class="feature-row header">
                <span>Feature</span>
                <span>CLASP</span>
                <span>OSC</span>
                <span>MQTT</span>
              </div>
              <div class="feature-row" v-for="f in benchmarks.features" :key="f.feature">
                <span>{{ f.feature }}</span>
                <span :class="{ yes: f.clasp === true || f.clasp === '3' }">{{ f.clasp === true ? 'Yes' : f.clasp === false ? 'No' : f.clasp }}</span>
                <span :class="{ yes: f.osc === true }">{{ f.osc === true ? 'Yes' : f.osc === false ? 'No' : f.osc }}</span>
                <span :class="{ yes: f.mqtt === true || f.mqtt === '3' }">{{ f.mqtt === true ? 'Yes' : f.mqtt === false ? 'No' : f.mqtt }}</span>
              </div>
            </div>

            <p style="margin-top: 1.5rem;"><b>Raw Performance (serialization only, no network):</b></p>

            <div class="bench-grid">
              <div class="bench-card">
                <div class="bench-title">Encoding Speed</div>
                <div class="bench-rows">
                  <div v-for="b in benchmarks.encoding" :key="b.proto" class="bench-row" :class="{ winner: b.winner }">
                    <span class="proto">{{ b.proto }}</span>
                    <span class="rate">{{ b.rate }} msg/s</span>
                  </div>
                </div>
              </div>
              <div class="bench-card">
                <div class="bench-title">Decoding Speed</div>
                <div class="bench-rows">
                  <div v-for="b in benchmarks.decoding" :key="b.proto" class="bench-row" :class="{ winner: b.winner }">
                    <span class="proto">{{ b.proto }}</span>
                    <span class="rate">{{ b.rate }} msg/s</span>
                  </div>
                </div>
              </div>
              <div class="bench-card">
                <div class="bench-title">Message Size</div>
                <div class="bench-rows">
                  <div v-for="b in benchmarks.size" :key="b.proto" class="bench-row" :class="{ winner: b.winner }">
                    <span class="proto">{{ b.proto }}</span>
                    <span class="rate">{{ b.bytes }} bytes</span>
                  </div>
                </div>
              </div>
            </div>

            <p class="bench-note">
              <b>Bottom line:</b> These are <em>codec-only</em> benchmarks (single core, in-memory, no routing/state/fanout). 
              Real system throughput is 10-100x lower. Run <code>real_benchmarks</code> for actual end-to-end numbers.
            </p>

            <p class="bench-run">
              Run benchmarks yourself: <code>cargo run -p clasp-test-suite --bin proof-tests --release</code>
            </p>
          </div>
        </section>
      </article>
    </div>
  </section>
</template>
