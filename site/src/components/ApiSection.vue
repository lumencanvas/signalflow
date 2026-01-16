<script setup>
import { ref, onMounted, watch, nextTick } from 'vue'
import CodeBlock from './CodeBlock.vue'

const activeTab = ref('js')

const tabs = [
  { id: 'js', label: 'JavaScript' },
  { id: 'py', label: 'Python' },
  { id: 'rs', label: 'Rust' },
  { id: 'c', label: 'C (Embedded)' }
]

// Accurate SDK examples based on actual codebase implementation
const jsCode = `// Install
npm i clasp

// Connect via WebSocket
import { Clasp } from 'clasp';
const clasp = new Clasp('wss://localhost:7330');

// Subscribe with wildcards
clasp.subscribe('/lumen/scene/*/layer/*/opacity', (value, addr, meta) => {
  console.log(addr, value);
});

// Set a Param (stateful, revisioned)
clasp.set('/lumen/scene/0/layer/0/opacity', 0.5);

// Emit an Event (ephemeral)
clasp.emit('/lumen/cue/fire', { cue: 'intro' });

// Send high-rate Stream data
clasp.stream('/controller/fader/1', 0.75);

// Schedule a Bundle for synchronized execution
clasp.bundle([
  { set: ['/light/1/intensity', 1.0] },
  { set: ['/light/2/intensity', 0.5] },
  { emit: ['/cue/fire', { id: 'intro' }] }
], { at: clasp.time() + 100000 }); // 100ms in the future

// Get current state snapshot
const state = await clasp.snapshot('/lumen/**');`

const pyCode = `# Install
pip install clasp

from clasp import Clasp, SignalType
import asyncio

async def main():
    clasp = Clasp('wss://localhost:7330')
    await clasp.connect()

    # Subscribe with callback
    @clasp.on('/lumen/scene/*/layer/*/opacity')
    def on_opacity(value, address, meta=None):
        print(f"{address} = {value}")

    # Set a Param
    await clasp.set('/lumen/scene/0/layer/0/opacity', 0.5)

    # Emit an Event
    await clasp.emit('/lumen/cue/fire', {'cue': 'intro'})

    # Scheduled bundle
    await clasp.bundle([
        ('set', '/light/1/intensity', 1.0),
        ('emit', '/cue/fire', {'id': 'intro'})
    ], at=clasp.time() + 100_000)

    await clasp.run()

asyncio.run(main())`

const rsCode = `// Cargo.toml
// clasp-client = "0.2"

use clasp_client::{Client, Message, Value};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to CLASP router
    let mut client = Client::connect("wss://localhost:7330").await?;

    // Subscribe with pattern matching
    client.subscribe("/lumen/scene/*/layer/*/opacity", |addr, val, meta| {
        println!("{} = {:?}", addr, val);
    }).await?;

    // Set a Param
    client.set("/lumen/scene/0/layer/0/opacity", Value::Float(0.5)).await?;

    // Publish an Event
    client.publish("/lumen/cue/fire", Value::Map(vec![
        ("cue".into(), Value::String("intro".into()))
    ].into_iter().collect())).await?;

    // Scheduled bundle
    let now = client.time();
    client.bundle(vec![
        Message::Set {
            address: "/light/1/intensity".into(),
            value: Value::Float(1.0),
        },
    ], Some(now + 100_000)).await?;

    client.run().await
}`

const cCode = `// CLASP Embedded SDK (no_std compatible)
// Uses UDP Lite profile with fixed 2-byte addresses
#include "clasp.h"

int main() {
    // Initialize UDP transport
    clasp_ctx_t* ctx = clasp_init_udp("192.168.1.42", 7331);

    // Set a Param (float32)
    clasp_set_f32(ctx, 0x0001, 0.75f);  // /controller/fader/1

    // Set a Param (integer)
    clasp_set_i32(ctx, 0x0100, 255);    // /dmx/0/1

    // Emit an Event
    uint8_t payload[] = {0x01, 0x00};
    clasp_emit(ctx, 0x1000, payload, sizeof(payload));

    // Poll for incoming messages
    clasp_msg_t msg;
    while (1) {
        if (clasp_recv(ctx, &msg, 100)) {  // 100ms timeout
            switch (msg.type) {
                case CLASP_MSG_SET:
                    printf("SET %04x = %f\\n", msg.address, msg.value.f32);
                    break;
                case CLASP_MSG_EVENT:
                    printf("EVENT %04x\\n", msg.address);
                    break;
            }
        }
    }

    clasp_free(ctx);
    return 0;
}`

const codeExamples = {
  js: { code: jsCode, lang: 'javascript' },
  py: { code: pyCode, lang: 'python' },
  rs: { code: rsCode, lang: 'rust' },
  c: { code: cCode, lang: 'c' }
}
</script>

<template>
  <section class="section" id="api">
    <h2>API & SDKs</h2>

    <div class="tabs" style="max-width: 900px; margin: 0 auto;">
      <div class="tab-bar">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="tab-btn"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          {{ tab.label }}
        </button>
      </div>

      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="tab-panel"
        :class="{ active: activeTab === tab.id }"
      >
        <CodeBlock
          :code="codeExamples[tab.id].code"
          :language="codeExamples[tab.id].lang"
        />
      </div>
    </div>
  </section>
</template>
