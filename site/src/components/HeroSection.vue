<script setup>
import ClaspLogo from './ClaspLogo.vue'

const emit = defineEmits(['scroll-to-spec'])

function scrollToSpec() {
  document.getElementById('spec')?.scrollIntoView({ behavior: 'smooth' })
}
</script>

<template>
  <section class="hero" id="top">
    <div class="lockup">
      <div class="logo">
        <ClaspLogo />
      </div>

      <h1>CLASP</h1>
      <div class="tagline">Creative Low-Latency Application Streaming Protocol</div>

      <div class="meta">
        WebSocket · WebRTC · QUIC · UDP · BLE<br/>
        Param · Event · Stream · Gesture · Timeline<br/>
        Discovery · State · Deterministic Time · Bridges · Security
      </div>

      <button class="cta" @click="scrollToSpec">READ THE FULL SPEC</button>
    </div>
  </section>
</template>
