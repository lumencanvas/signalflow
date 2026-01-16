<script setup>
import { ref } from 'vue'
import CodeBlock from './CodeBlock.vue'

const activeSection = ref('eli5')

const specSections = ref([
  { id: 'eli5', title: '0. Explain Like I\'m 5', open: true },
  { id: 'principles', title: '1. Design Principles', open: false },
  { id: 'transport', title: '2. Transport Layer', open: false },
  { id: 'frame', title: '3. Frame & Payload', open: false },
  { id: 'discovery', title: '4. Discovery', open: false },
  { id: 'signals', title: '5. Signal Types', open: false },
  { id: 'messages', title: '6. Messages', open: false },
  { id: 'types', title: '7. Data Types', open: false },
  { id: 'security', title: '8. Security', open: false },
  { id: 'bridges', title: '9. Bridges', open: false },
  { id: 'timing', title: '10. Timing', open: false },
  { id: 'conformance', title: '11. Conformance', open: false }
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

// Code examples - accurate to actual implementation
const transportCode = `// WebSocket (MUST support)
const ws = new WebSocket('wss://localhost:7330/clasp', 'clasp.v2');
ws.binaryType = 'arraybuffer';

// WebRTC DataChannel (SHOULD support for P2P)
const dc = pc.createDataChannel('clasp', {
  ordered: false,
  maxRetransmits: 0
});

// UDP (MAY support for LAN low-latency)
// QUIC (MAY support via quinn library)`

const frameCode = `Byte 0:     Magic 0x53 ('S')
Byte 1:     Flags
            [7:6] QoS (00=fire, 01=confirm, 10=commit, 11=reserved)
            [5]   Timestamp present
            [4]   Encrypted
            [3]   Compressed
            [2:0] Reserved
Byte 2-3:   Payload Length (uint16 big-endian, max 65535)
[Optional]  Bytes 4-11: Timestamp (uint64 microseconds)
Payload:    MessagePack encoded message`

const discoveryCode = `// mDNS TXT record format
{
  "version": "2",
  "name": "CLASP Studio",
  "features": "psetg",  // p=param, s=stream, e=event, t=timeline, g=gesture
  "ws": "7330"
}

// UDP broadcast fallback on port 7331
// HELLO broadcast -> ANNOUNCE unicast response`

const paramExample = `// Param message structure (actual implementation)
{
  "type": "SET",
  "address": "/lumen/scene/0/layer/3/opacity",
  "value": 0.75,
  "revision": 42,
  "writer": "session:abc123",
  "timestamp": 1704067200000000,
  "meta": {
    "unit": "normalized",
    "range": [0, 1],
    "default": 1.0
  }
}`

const subscribeExample = `// SUBSCRIBE with stream options
{
  "type": "SUBSCRIBE",
  "id": 1,
  "pattern": "/controller/fader/*",
  "types": ["stream"],
  "options": {
    "maxRate": 30,
    "epsilon": 0.01,
    "window": 100
  }
}`

const bundleExample = `// Scheduled bundle (atomic execution)
{
  "type": "BUNDLE",
  "timestamp": 1704067300000000,
  "messages": [
    { "type": "SET", "address": "/light/1/intensity", "value": 1.0 },
    { "type": "PUBLISH", "address": "/cue/fire", "payload": { "id": "intro" } }
  ]
}`

const dataTypesCode = `// MessagePack Extension Types for Creative Primitives
Ext 0x10  vec2    (f32×2)   - 2D position/UV
Ext 0x11  vec3    (f32×3)   - 3D position/RGB
Ext 0x12  vec4    (f32×4)   - 4D position/RGBA
Ext 0x13  color   (u8×4)    - RGBA 8-bit color
Ext 0x14  colorf  (f32×4)   - RGBA float color
Ext 0x15  mat3    (f32×9)   - 3x3 matrix
Ext 0x16  mat4    (f32×16)  - 4x4 matrix

// Native MessagePack types
Null, Bool, Int (i64), Float (f64),
String, Bytes (Vec<u8>), Array, Map`

const capabilityToken = `// JWT capability token claims
{
  "sub": "user:moheeb",
  "clasp": {
    "read": ["/lumen/**"],
    "write": ["/lumen/scene/*/layer/*/opacity"],
    "constraints": {
      "/lumen/scene/*/layer/*/opacity": {
        "range": [0, 1],
        "maxRate": 60
      }
    }
  }
}`

const bridgesCode = `// Bridge address mappings (actual implementation)
MIDI CC          → Param  /midi/{device}/cc/{channel}/{num}
MIDI Note On/Off → Event  /midi/{device}/note/{channel}
MIDI Pitchbend   → Param  /midi/{device}/pitchbend/{channel}
OSC /path        → SET    address preserves OSC path
Art-Net Universe → Param  /artnet/{universe}/{channel}
DMX Universe     → Param  /dmx/{universe}/{channel}
sACN Universe    → Param  /sacn/{universe}/{channel}`

const timingCode = `// NTP-style clock synchronization
roundTrip = (T4 - T1) - (T3 - T2)
offset    = ((T2 - T1) + (T3 - T4)) / 2

// Sync message exchange
Client → Server: SYNC { t1: client_send_time }
Server → Client: SYNC { t1, t2: server_recv_time, t3: server_send_time }
Client calculates: t4 = client_recv_time

// Scheduled bundle execution target: ±1ms
// Jitter buffer recommendation: 20-50ms for WiFi/WAN`

const messages = [
  { name: 'HELLO / WELCOME', desc: 'Handshake + session assignment + capability negotiation' },
  { name: 'ANNOUNCE', desc: 'Advertise namespace + signals + capabilities' },
  { name: 'SUBSCRIBE / UNSUBSCRIBE', desc: 'Pattern subscriptions with wildcards (* and **)' },
  { name: 'SET', desc: 'Write Param (stateful, revisioned)' },
  { name: 'GET / SNAPSHOT', desc: 'Read state / dump all current values' },
  { name: 'PUBLISH', desc: 'Send Event/Stream/Gesture payloads' },
  { name: 'BUNDLE', desc: 'Atomic group with optional scheduled timestamp' },
  { name: 'SYNC', desc: 'Clock synchronization (NTP-style)' },
  { name: 'PING / PONG', desc: 'Keep-alive and latency measurement' },
  { name: 'ACK / ERROR', desc: 'Delivery confirmation + error reporting' },
  { name: 'QUERY / RESULT', desc: 'Introspection and discovery' }
]

const signalTypes = [
  { name: 'Param', desc: 'Stateful, revisioned. Default QoS: Confirm. Supports conflict strategies.' },
  { name: 'Event', desc: 'Ephemeral trigger. Default QoS: Confirm. Fire-and-forget semantics.' },
  { name: 'Stream', desc: 'High-rate samples. Default QoS: Fire. Supports downsampling/epsilon.' },
  { name: 'Gesture', desc: 'Phased input (start/move/end). Move events may be coalesced.' },
  { name: 'Timeline', desc: 'Immutable automation lanes. Default QoS: Commit. Time-indexed.' }
]

const conformanceLevels = [
  { level: 'Minimal', reqs: 'WebSocket + HELLO/WELCOME + SET/PUBLISH + MessagePack' },
  { level: 'Standard', reqs: '+ SUBSCRIBE + Param/Event/Stream + full QoS support' },
  { level: 'Full', reqs: '+ Timeline + Gesture + Discovery (mDNS/UDP) + Bridges' },
  { level: 'Embedded', reqs: 'UDP + numeric IDs (2-byte) + fixed-size messages (no_std)' }
]
</script>

<template>
  <section class="section" id="spec">
    <h2>FULL SPEC (CLASP v2)</h2>

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
        <!-- 0. ELI5 -->
        <section
          :id="`spec-eli5`"
          class="spec-section"
          :class="{ open: specSections[0].open }"
        >
          <h3 @click="toggleSection(specSections[0])">0. Explain Like I'm 5</h3>
          <div class="spec-content">
            <p>
              CLASP is a universal language so creative tools can talk: keyboards, lights, visuals,
              sensors, and apps. It's fast, remembers the current settings, keeps everything
              synchronized, works on LAN or internet, and can translate older protocols like MIDI/OSC/DMX.
            </p>
          </div>
        </section>

        <!-- 1. Principles -->
        <section
          :id="`spec-principles`"
          class="spec-section"
          :class="{ open: specSections[1].open }"
        >
          <h3 @click="toggleSection(specSections[1])">1. Design Principles</h3>
          <div class="spec-content">
            <ul>
              <li><b>Browser-native:</b> WebSocket MUST work. (WebRTC SHOULD; everything else optional.)</li>
              <li><b>Progressive enhancement:</b> Hello → send signal → done, in a few lines.</li>
              <li><b>Semantic signals:</b> Param/Event/Stream/Gesture/Timeline are protocol primitives, not conventions.</li>
              <li><b>Discovery first-class:</b> mDNS auto-discovery; UDP broadcast fallback; manual/QR when needed.</li>
              <li><b>State is truth:</b> Params have authoritative values with revisions and conflict strategies.</li>
              <li><b>Timing deterministic:</b> Scheduled bundles + NTP-style clock sync are built-in.</li>
              <li><b>Security without ceremony:</b> TLS/DTLS encrypted by default; capability tokens for auth.</li>
              <li><b>Legacy respected:</b> Bridges for MIDI/OSC/DMX/Art-Net/sACN are defined in the spec.</li>
            </ul>
          </div>
        </section>

        <!-- 2. Transport -->
        <section
          :id="`spec-transport`"
          class="spec-section"
          :class="{ open: specSections[2].open }"
        >
          <h3 @click="toggleSection(specSections[2])">2. Transport Layer</h3>
          <div class="spec-content">
            <p><b>Priority order:</b> WebSocket (MUST) · WebRTC DataChannel (SHOULD) · QUIC/HTTP3 (MAY) · UDP (MAY) · BLE (MAY) · Serial (MAY).</p>
            <p><b>Default ports:</b> WebSocket 7330 · Discovery UDP 7331</p>
            <p><b>Subprotocol:</b> <code>clasp.v2</code></p>
            <CodeBlock :code="transportCode" language="javascript" />
          </div>
        </section>

        <!-- 3. Frame -->
        <section
          :id="`spec-frame`"
          class="spec-section"
          :class="{ open: specSections[3].open }"
        >
          <h3 @click="toggleSection(specSections[3])">3. Frame & Payload</h3>
          <div class="spec-content">
            <p>CLASP uses a minimal binary frame (4 bytes overhead, 12 with timestamp) with a MessagePack payload.</p>
            <CodeBlock :code="frameCode" language="plaintext" />
            <p style="margin-top: 1rem;"><b>QoS Levels:</b></p>
            <ul>
              <li><b>Fire (00):</b> Best-effort, no confirmation. Used for high-rate streams.</li>
              <li><b>Confirm (01):</b> At-least-once delivery with ACK. Default for Params/Events.</li>
              <li><b>Commit (10):</b> Exactly-once, ordered delivery. Used for Timelines and transactions.</li>
            </ul>
          </div>
        </section>

        <!-- 4. Discovery -->
        <section
          :id="`spec-discovery`"
          class="spec-section"
          :class="{ open: specSections[4].open }"
        >
          <h3 @click="toggleSection(specSections[4])">4. Discovery</h3>
          <div class="spec-content">
            <p>Three mechanisms, in priority order:</p>
            <ol>
              <li><b>mDNS:</b> <code>_clasp._tcp.local</code> with TXT records for version/features/port.</li>
              <li><b>UDP broadcast fallback:</b> HELLO on port 7331; ANNOUNCE unicast response.</li>
              <li><b>WAN rendezvous:</b> Simple register/discover API for public endpoints.</li>
            </ol>
            <CodeBlock :code="discoveryCode" language="json" />
            <p><b>Browser limitation:</b> Browsers can't do mDNS/UDP; they connect to a known WSS endpoint (manual or QR) which can relay discovery info.</p>
          </div>
        </section>

        <!-- 5. Signal Types -->
        <section
          :id="`spec-signals`"
          class="spec-section"
          :class="{ open: specSections[5].open }"
        >
          <h3 @click="toggleSection(specSections[5])">5. Signal Types</h3>
          <div class="spec-content">
            <div class="spec-grid">
              <div v-for="sig in signalTypes" :key="sig.name">
                <b>{{ sig.name }}</b><br/>
                <span>{{ sig.desc }}</span>
              </div>
            </div>
            <CodeBlock :code="paramExample" language="json" />
            <p style="margin-top: 1rem;"><b>Conflict Resolution Strategies:</b></p>
            <ul>
              <li><b>LWW (default):</b> Last-write-wins by timestamp</li>
              <li><b>Max:</b> Keep maximum value</li>
              <li><b>Min:</b> Keep minimum value</li>
              <li><b>Lock:</b> First writer holds lock until released</li>
              <li><b>Merge:</b> Application-defined merge function</li>
            </ul>
          </div>
        </section>

        <!-- 6. Messages -->
        <section
          :id="`spec-messages`"
          class="spec-section"
          :class="{ open: specSections[6].open }"
        >
          <h3 @click="toggleSection(specSections[6])">6. Messages</h3>
          <div class="spec-content">
            <p>MessagePack payloads are maps with a <code>type</code> field (string or numeric code). Core catalog:</p>
            <div class="table">
              <div class="row head">
                <div>Message</div>
                <div>Purpose</div>
              </div>
              <div class="row" v-for="msg in messages" :key="msg.name">
                <div>{{ msg.name }}</div>
                <div>{{ msg.desc }}</div>
              </div>
            </div>
            <CodeBlock :code="subscribeExample" language="json" />
            <CodeBlock :code="bundleExample" language="json" />
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
            <p>Scalar types use native MessagePack. Creative primitives use MessagePack extension types.</p>
            <CodeBlock :code="dataTypesCode" language="plaintext" />
          </div>
        </section>

        <!-- 8. Security -->
        <section
          :id="`spec-security`"
          class="spec-section"
          :class="{ open: specSections[8].open }"
        >
          <h3 @click="toggleSection(specSections[8])">8. Security</h3>
          <div class="spec-content">
            <ul>
              <li><b>Open:</b> Development / trusted LAN mode.</li>
              <li><b>Encrypted:</b> WSS (TLS 1.3) for WebSocket; DTLS for UDP/WebRTC.</li>
              <li><b>Authenticated:</b> Capability tokens (JWT) with read/write scopes + constraints.</li>
            </ul>
            <CodeBlock :code="capabilityToken" language="json" />
            <p><b>Zero-config pairing:</b> Server shows a short code or QR; clients derive a shared secret for encrypted sessions without PKI.</p>
          </div>
        </section>

        <!-- 9. Bridges -->
        <section
          :id="`spec-bridges`"
          class="spec-section"
          :class="{ open: specSections[9].open }"
        >
          <h3 @click="toggleSection(specSections[9])">9. Bridges</h3>
          <div class="spec-content">
            <p>Bridges are CLASP nodes that translate legacy protocols into semantic signals. All bridges support bidirectional conversion.</p>
            <CodeBlock :code="bridgesCode" language="plaintext" />
            <p style="margin-top: 1rem;"><b>Currently Implemented:</b></p>
            <ul>
              <li><b>OSC:</b> Full bidirectional with bundle support and timestamp preservation</li>
              <li><b>MIDI:</b> Input/output ports, CC, notes, program change, pitchbend</li>
              <li><b>Art-Net:</b> Multiple universes, delta detection, polling</li>
              <li><b>DMX-512:</b> ENTTEC Pro/Open, generic FTDI, 44Hz default refresh</li>
              <li><b>sACN (E1.31):</b> Multicast/unicast support</li>
            </ul>
          </div>
        </section>

        <!-- 10. Timing -->
        <section
          :id="`spec-timing`"
          class="spec-section"
          :class="{ open: specSections[10].open }"
        >
          <h3 @click="toggleSection(specSections[10])">10. Timing</h3>
          <div class="spec-content">
            <p>CLASP uses NTP-style SYNC messages to estimate offset + RTT + jitter. Clients should resync periodically (recommended: every 30s).</p>
            <CodeBlock :code="timingCode" language="plaintext" />
          </div>
        </section>

        <!-- 11. Conformance -->
        <section
          :id="`spec-conformance`"
          class="spec-section"
          :class="{ open: specSections[11].open }"
        >
          <h3 @click="toggleSection(specSections[11])">11. Conformance Levels</h3>
          <div class="spec-content">
            <div class="table">
              <div class="row head">
                <div>Level</div>
                <div>Requirements</div>
              </div>
              <div class="row" v-for="level in conformanceLevels" :key="level.level">
                <div>{{ level.level }}</div>
                <div>{{ level.reqs }}</div>
              </div>
            </div>
          </div>
        </section>
      </article>
    </div>
  </section>
</template>
