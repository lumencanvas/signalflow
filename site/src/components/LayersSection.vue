<script setup>
import { ref } from 'vue'
import CodeBlock from './CodeBlock.vue'

const layers = ref([
  {
    id: 'eli5',
    title: "EXPLAIN LIKE I'M 5",
    layer: 'Layer 0',
    open: false,
    content: {
      type: 'text',
      text: 'CLASP is a universal language so music gear, lights, visuals, and apps can talk to each other. It keeps everything in sync, remembers state, and works over the internet or locally.'
    }
  },
  {
    id: 'technical',
    title: 'TECHNICAL OVERVIEW',
    layer: 'Layer 1',
    open: false,
    content: {
      type: 'list',
      items: [
        'Semantic signal types: Param, Event, Stream, Gesture, Timeline',
        'State with revisioning and conflict strategies',
        'Clock sync and scheduled bundles',
        'Discovery: mDNS, LAN fallback, WAN rendezvous',
        'Security: TLS/DTLS, capability tokens'
      ]
    }
  },
  {
    id: 'wire',
    title: 'WIRE FORMAT',
    layer: 'Layer 2',
    open: false,
    content: {
      type: 'code',
      lang: 'plaintext',
      code: `Byte 0   Magic 0x53 ('S')
Byte 1   Flags (QoS, Timestamp, Crypto)
Byte 2-3 Payload Length (u16 big-endian)
Byte 4+  Optional Timestamp (u64 microseconds)
Payload  MessagePack`
    }
  }
])

function toggle(layer) {
  layer.open = !layer.open
}
</script>

<template>
  <section class="section" id="layers">
    <h2>PROTOCOL LAYERS</h2>

    <div class="layers">
      <div
        v-for="layer in layers"
        :key="layer.id"
        class="layer"
        :class="{ open: layer.open }"
      >
        <div class="layer-header" @click="toggle(layer)">
          {{ layer.title }} <span>{{ layer.layer }}</span>
        </div>
        <div class="layer-content">
          <div class="layer-body">
            <p v-if="layer.content.type === 'text'">{{ layer.content.text }}</p>
            <ul v-else-if="layer.content.type === 'list'">
              <li v-for="item in layer.content.items" :key="item">{{ item }}</li>
            </ul>
            <CodeBlock
              v-else-if="layer.content.type === 'code'"
              :code="layer.content.code"
              :language="layer.content.lang"
            />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
