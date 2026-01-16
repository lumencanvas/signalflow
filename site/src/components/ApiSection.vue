<script setup>
import { ref, onMounted, watch, nextTick } from 'vue'
import CodeBlock from './CodeBlock.vue'

const activeTab = ref('js')

const tabs = [
  { id: 'js', label: 'JavaScript' },
  { id: 'py', label: 'Python' },
  { id: 'rs', label: 'Rust' }
]

// SDK examples matching actual implementation
const jsCode = `// Install: npm install @clasp-to/core

import { Clasp } from '@clasp-to/core';

// Connect to a CLASP router
const clasp = new Clasp('ws://localhost:7330');
await clasp.connect();

// Subscribe to addresses (wildcards supported)
const unsubscribe = clasp.on('/lights/*/brightness', (value, address) => {
  console.log(\`\${address} = \${value}\`);
});

// Set a Param (stateful, syncs to all subscribers)
clasp.set('/lights/kitchen/brightness', 0.75);

// Get current value (from cache or server)
const brightness = await clasp.get('/lights/kitchen/brightness');

// Emit an Event (one-shot, no state)
clasp.emit('/cue/fire', { cueId: 'intro', fadeTime: 2.0 });

// Send Stream data (high-rate, fire-and-forget)
clasp.stream('/sensors/accelerometer/x', 0.342);

// Atomic bundle with optional scheduling
clasp.bundle([
  { set: ['/light/1/intensity', 1.0] },
  { set: ['/light/2/intensity', 0.5] },
  { emit: ['/cue/fire', { id: 'intro' }] }
], { at: clasp.time() + 100000 }); // 100ms in the future

// Cleanup
unsubscribe();
clasp.close();`

const pyCode = `# Install: pip install clasp-to

import asyncio
from clasp import ClaspBuilder

async def main():
    # Connect using builder pattern
    client = await (
        ClaspBuilder('ws://localhost:7330')
        .with_name('Python Controller')
        .connect()
    )

    # Subscribe with decorator
    @client.on('/lights/*/brightness')
    def on_brightness(value, address, meta=None):
        print(f"{address} = {value}")

    # Set a Param
    await client.set('/lights/kitchen/brightness', 0.75)

    # Emit an Event
    await client.emit('/cue/fire', {'cueId': 'intro'})

    # Get current value
    brightness = await client.get('/lights/kitchen/brightness')
    print(f"Current: {brightness}")

    # Keep running (processes incoming messages)
    await client.run()

asyncio.run(main())`

const rsCode = `// Cargo.toml: clasp-client = "0.1"

use clasp_client::{Clasp, ClaspBuilder};
use clasp_core::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect using builder
    let client = ClaspBuilder::new("ws://localhost:7330")
        .name("Rust Controller")
        .connect()
        .await?;

    // Subscribe with pattern matching
    let _unsub = client.subscribe("/lights/**", |value, address| {
        println!("{} = {:?}", address, value);
    }).await?;

    // Set a Param
    client.set("/lights/kitchen/brightness", Value::Float(0.75)).await?;

    // Emit an Event
    client.emit("/cue/fire", Value::Map(
        [("cueId".into(), Value::String("intro".into()))].into()
    )).await?;

    // Scheduled bundle
    let now = client.time();
    client.bundle(vec![
        clasp_core::Message::Set(clasp_core::SetMessage {
            address: "/light/1/intensity".into(),
            value: Value::Float(1.0),
            ..Default::default()
        }),
    ], Some(now + 100_000)).await?;

    // Run until Ctrl-C
    tokio::signal::ctrl_c().await?;
    client.close().await?;
    Ok(())
}`

const codeExamples = {
  js: { code: jsCode, lang: 'javascript' },
  py: { code: pyCode, lang: 'python' },
  rs: { code: rsCode, lang: 'rust' }
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
