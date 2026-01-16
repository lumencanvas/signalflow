<script setup>
import { ref, computed, onMounted } from 'vue'
import { useClasp } from '../../composables/useClasp'
import CodeSnippet from './CodeSnippet.vue'

const { connected, sessionId, settings } = useClasp()

const discoveryEnabled = ref(false)
const serviceName = ref('Playground Client')
const customPort = ref(7330)

// Generate a unique ID for this browser instance
const instanceId = ref('')

onMounted(() => {
  instanceId.value = Math.random().toString(36).substring(2, 10)
})

const mdnsRecord = computed(() => {
  return {
    name: `${serviceName.value}-${instanceId.value}`,
    type: '_clasp._tcp.local.',
    port: customPort.value,
    txt: {
      version: '2',
      name: serviceName.value,
      features: 'param,event,stream',
      session: sessionId.value || 'not-connected',
    },
  }
})

const mdnsRecordFormatted = computed(() => {
  return JSON.stringify(mdnsRecord.value, null, 2)
})

function toggleDiscovery() {
  discoveryEnabled.value = !discoveryEnabled.value
  // In a real implementation, this would register/unregister with mDNS
  // Browsers can't directly use mDNS, but we show the concept
}

function copyConnectionUrl() {
  const url = settings.url || 'ws://localhost:7330'
  navigator.clipboard.writeText(url)
}
</script>

<template>
  <div class="discovery-tab">
    <div class="discovery-header">
      <h3>Service Discovery</h3>
      <p class="hint">
        CLASP uses mDNS/DNS-SD for zero-configuration service discovery on local networks.
        Desktop apps can scan for CLASP servers automatically.
      </p>
    </div>

    <div class="discovery-grid">
      <!-- How Discovery Works -->
      <div class="discovery-card full-width">
        <h4>How Discovery Works</h4>

        <div class="flow-diagram">
          <div class="flow-step">
            <div class="step-number">1</div>
            <div class="step-content">
              <div class="step-title">Server Announces</div>
              <div class="step-desc">
                CLASP servers broadcast their presence using mDNS on the
                <code>_clasp._tcp.local.</code> service type.
              </div>
            </div>
          </div>

          <div class="flow-arrow">→</div>

          <div class="flow-step">
            <div class="step-number">2</div>
            <div class="step-content">
              <div class="step-title">Client Scans</div>
              <div class="step-desc">
                Desktop/mobile apps browse for <code>_clasp._tcp</code> services
                on the local network.
              </div>
            </div>
          </div>

          <div class="flow-arrow">→</div>

          <div class="flow-step">
            <div class="step-number">3</div>
            <div class="step-content">
              <div class="step-title">Auto-Connect</div>
              <div class="step-desc">
                App displays discovered servers with their name, version, and features.
                User selects to connect.
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- mDNS Record Structure -->
      <div class="discovery-card">
        <h4>mDNS Record Structure</h4>
        <p class="card-hint">
          Each CLASP server publishes a service record with connection details.
        </p>

        <div class="record-config">
          <div class="field">
            <label>Service Name</label>
            <input v-model="serviceName" type="text" placeholder="My CLASP Server" />
          </div>

          <div class="field">
            <label>Port</label>
            <input v-model.number="customPort" type="number" min="1" max="65535" />
          </div>
        </div>

        <div class="record-preview">
          <div class="preview-label">DNS-SD Record</div>
          <pre>{{ mdnsRecordFormatted }}</pre>
        </div>

        <CodeSnippet :code="`// Server-side: Announce via mDNS
server.announce({
  name: '${serviceName.value}',
  type: '_clasp._tcp.local.',
  port: ${customPort},
  txt: {
    version: '2',
    features: 'param,event,stream'
  }
});

// Client-side: Browse for services
const services = await clasp.discover();
// Returns array of discovered servers`" />
      </div>

      <!-- Try Discovery -->
      <div class="discovery-card">
        <h4>Try Discovery</h4>
        <p class="card-hint">
          Use the CLASP desktop app to scan for servers on your network.
        </p>

        <div class="try-steps">
          <div class="try-step">
            <span class="step-num">1</span>
            <span>Download and install CLASP Desktop App</span>
          </div>
          <div class="try-step">
            <span class="step-num">2</span>
            <span>Start a local CLASP server (or use the public relay)</span>
          </div>
          <div class="try-step">
            <span class="step-num">3</span>
            <span>In the app, click "Scan" to discover servers</span>
          </div>
          <div class="try-step">
            <span class="step-num">4</span>
            <span>Select a server to connect</span>
          </div>
        </div>

        <div class="current-connection">
          <div class="conn-label">Current Connection URL</div>
          <div class="conn-url">
            <code>{{ settings.url }}</code>
            <button @click="copyConnectionUrl" class="copy-btn">Copy</button>
          </div>
          <p class="conn-hint">
            Share this URL with others to let them connect to the same server.
          </p>
        </div>
      </div>

      <!-- Browser Limitations -->
      <div class="discovery-card">
        <h4>Browser Limitations</h4>
        <p class="card-hint">
          Browsers cannot directly use mDNS for security reasons.
        </p>

        <div class="limitations">
          <div class="limitation warning">
            <div class="limit-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/>
                <line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
            </div>
            <div class="limit-text">
              <strong>No mDNS Access:</strong> Browsers sandbox network access and cannot browse
              local mDNS services directly.
            </div>
          </div>

          <div class="limitation success">
            <div class="limit-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </div>
            <div class="limit-text">
              <strong>WebSocket Works:</strong> Once you have a server address, browsers can connect
              via WebSocket just fine.
            </div>
          </div>

          <div class="limitation info">
            <div class="limit-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="16" x2="12" y2="12"/>
                <line x1="12" y1="8" x2="12.01" y2="8"/>
              </svg>
            </div>
            <div class="limit-text">
              <strong>Workaround:</strong> Use the desktop app for discovery, or manually enter
              the server URL in the browser.
            </div>
          </div>
        </div>

        <CodeSnippet :code="`// In desktop/mobile apps (Node.js, Electron, native):
import { discover } from '@clasp-to/discovery';

const servers = await discover();
// [{ name: 'Studio A', host: '192.168.1.50', port: 7330 }]

// In browsers, manually connect:
const client = await ClaspBuilder('ws://192.168.1.50:7330')
  .connect();`" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.discovery-tab {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.discovery-header h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
  letter-spacing: 0.15em;
}

.discovery-header .hint {
  margin: 0;
  font-size: 0.85rem;
  opacity: 0.6;
  line-height: 1.5;
}

.discovery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 1rem;
}

.discovery-card {
  border: 1px solid rgba(0,0,0,0.12);
  padding: 1.2rem;
  background: rgba(255,255,255,0.4);
}

.discovery-card.full-width {
  grid-column: 1 / -1;
}

.discovery-card h4 {
  margin: 0 0 0.5rem;
  font-size: 0.9rem;
  letter-spacing: 0.12em;
}

.card-hint {
  margin: 0 0 1rem;
  font-size: 0.8rem;
  opacity: 0.6;
  line-height: 1.4;
}

/* Flow Diagram */
.flow-diagram {
  display: flex;
  align-items: stretch;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.flow-step {
  flex: 1;
  display: flex;
  gap: 0.8rem;
  padding: 1rem;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.08);
}

.step-number {
  width: 28px;
  height: 28px;
  background: var(--accent);
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.85rem;
  flex-shrink: 0;
}

.step-content {
  flex: 1;
}

.step-title {
  font-weight: 600;
  font-size: 0.85rem;
  margin-bottom: 0.3rem;
}

.step-desc {
  font-size: 0.8rem;
  opacity: 0.7;
  line-height: 1.4;
}

.step-desc code {
  background: rgba(0,0,0,0.06);
  padding: 0.1rem 0.3rem;
  font-size: 0.75rem;
}

.flow-arrow {
  display: flex;
  align-items: center;
  font-size: 1.5rem;
  opacity: 0.3;
}

/* Record Config */
.record-config {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 1rem;
  margin-bottom: 1rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.field label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  opacity: 0.6;
}

input {
  padding: 0.5rem 0.7rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.5);
  font-family: inherit;
  font-size: 0.85rem;
}

input:focus {
  outline: none;
  border-color: var(--accent);
}

.record-preview {
  background: rgba(0,0,0,0.03);
  padding: 0.8rem;
  margin-bottom: 1rem;
}

.preview-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.5;
  margin-bottom: 0.5rem;
}

.record-preview pre {
  margin: 0;
  font-size: 0.75rem;
  line-height: 1.5;
  overflow-x: auto;
}

/* Try Steps */
.try-steps {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  margin-bottom: 1.5rem;
}

.try-step {
  display: flex;
  align-items: center;
  gap: 0.8rem;
  font-size: 0.85rem;
}

.step-num {
  width: 22px;
  height: 22px;
  background: var(--ink);
  color: var(--paper);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  flex-shrink: 0;
}

.current-connection {
  border: 1px solid rgba(0,0,0,0.1);
  padding: 1rem;
  background: rgba(0,0,0,0.02);
}

.conn-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.5;
  margin-bottom: 0.5rem;
}

.conn-url {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}

.conn-url code {
  flex: 1;
  font-size: 0.85rem;
  padding: 0.5rem;
  background: rgba(255,255,255,0.5);
  border: 1px solid rgba(0,0,0,0.1);
}

.copy-btn {
  padding: 0.5rem 0.8rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.75rem;
  cursor: pointer;
}

.copy-btn:hover {
  background: var(--accent);
}

.conn-hint {
  margin: 0;
  font-size: 0.75rem;
  opacity: 0.5;
}

/* Limitations */
.limitations {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin-bottom: 1rem;
}

.limitation {
  display: flex;
  gap: 0.8rem;
  padding: 0.8rem;
  background: rgba(0,0,0,0.03);
}

.limit-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
}

.limit-icon svg {
  width: 100%;
  height: 100%;
}

.limitation.warning .limit-icon {
  color: #F59E0B;
}

.limitation.success .limit-icon {
  color: #10B981;
}

.limitation.info .limit-icon {
  color: #3B82F6;
}

.limit-text {
  font-size: 0.8rem;
  line-height: 1.5;
}

.limit-text strong {
  display: block;
  margin-bottom: 0.2rem;
}

@media (max-width: 900px) {
  .flow-diagram {
    flex-direction: column;
  }

  .flow-arrow {
    transform: rotate(90deg);
    justify-content: center;
  }

  .record-config {
    grid-template-columns: 1fr;
  }
}
</style>
