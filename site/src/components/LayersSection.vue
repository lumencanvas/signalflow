<script setup>
import CodeBlock from './CodeBlock.vue'

const quickCode = `const clasp = new Clasp('ws://localhost:7330');
await clasp.connect();

// Listen for any light changes
clasp.on('/lights/**', (value, addr) => console.log(addr, value));

// Set a value - syncs to all connected apps
clasp.set('/lights/main/brightness', 0.8);`

const bridges = [
  { name: 'MIDI', desc: 'CC, notes, pitchbend' },
  { name: 'OSC', desc: 'Full path & bundle support' },
  { name: 'Art-Net', desc: 'Multiple universes' },
  { name: 'DMX', desc: 'ENTTEC Pro/Open' },
  { name: 'MQTT', desc: 'v3.1.1 and v5' },
  { name: 'HTTP', desc: 'REST API' }
]
</script>

<template>
  <section class="section" id="layers">
    <div class="intro">
      <h2>UNIVERSAL PROTOCOL BRIDGE</h2>
      <p class="lead">
        Connect <b>everything</b>. Your MIDI controller talks to your DMX lights.
        Your OSC app controls your VJ software. All through one unified address space.
      </p>
    </div>

    <div class="features-grid">
      <div class="feature-card">
        <div class="feature-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        <h3>State That Syncs</h3>
        <p>Unlike OSC, CLASP tracks state. Late-joining clients get current values, not just future changes.</p>
      </div>

      <div class="feature-card">
        <div class="feature-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
          </svg>
        </div>
        <h3>Sub-ms Latency</h3>
        <p>Built for real-time. Stream sensor data at 60Hz, trigger cues in sync, schedule bundles to the microsecond.</p>
      </div>

      <div class="feature-card">
        <div class="feature-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 11a9 9 0 0 1 9 9M4 4a16 16 0 0 1 16 16"/><circle cx="5" cy="19" r="1"/>
          </svg>
        </div>
        <h3>Auto Discovery</h3>
        <p>Find CLASP routers on your network automatically via mDNS. No IP addresses to remember.</p>
      </div>
    </div>

    <div class="bridges-section">
      <h3>BUILT-IN BRIDGES</h3>
      <div class="bridges-grid">
        <div class="bridge" v-for="b in bridges" :key="b.name">
          <span class="bridge-name">{{ b.name }}</span>
          <span class="bridge-desc">{{ b.desc }}</span>
        </div>
      </div>
    </div>

    <div class="code-preview">
      <div class="code-label">5 LINES TO GET STARTED</div>
      <CodeBlock :code="quickCode" language="javascript" />
    </div>
  </section>
</template>

<style scoped>
.intro {
  max-width: 700px;
  margin: 0 auto 2rem;
  text-align: center;
}

.intro h2 {
  margin-bottom: 1rem;
}

.lead {
  font-size: 1.15rem;
  line-height: 1.6;
  opacity: 0.85;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 1.5rem;
  max-width: 900px;
  margin: 0 auto 2.5rem;
}

.feature-card {
  padding: 1.5rem;
  border: 1px solid rgba(0,0,0,0.12);
  background: rgba(255,255,255,0.3);
}

.feature-icon {
  color: var(--accent);
  margin-bottom: 0.75rem;
}

.feature-card h3 {
  font-size: 1rem;
  letter-spacing: 0.1em;
  margin-bottom: 0.5rem;
}

.feature-card p {
  font-size: 0.9rem;
  line-height: 1.5;
  opacity: 0.8;
  margin: 0;
}

.bridges-section {
  max-width: 700px;
  margin: 0 auto 2.5rem;
}

.bridges-section h3 {
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  margin-bottom: 1rem;
  text-align: center;
  opacity: 0.7;
}

.bridges-grid {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.75rem;
}

.bridge {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.4);
  font-size: 0.85rem;
}

.bridge-name {
  font-weight: 600;
  letter-spacing: 0.08em;
}

.bridge-desc {
  opacity: 0.6;
  font-size: 0.8rem;
}

.code-preview {
  max-width: 600px;
  margin: 0 auto;
}

.code-label {
  font-size: 0.7rem;
  letter-spacing: 0.2em;
  margin-bottom: 0.75rem;
  opacity: 0.6;
  text-align: center;
}

@media (max-width: 600px) {
  .bridge {
    flex-direction: column;
    gap: 0.2rem;
    text-align: center;
  }
}
</style>
