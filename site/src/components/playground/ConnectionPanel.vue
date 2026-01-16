<script setup>
import { onMounted } from 'vue'
import { useClasp } from '../../composables/useClasp'

const {
  connected,
  connecting,
  error,
  sessionId,
  settings,
  connect,
  disconnect,
  discoveredServers,
  scanning,
  scan,
} = useClasp()

const presets = [
  { label: 'Local Server', url: 'ws://localhost:7330' },
  { label: 'Public Relay', url: 'wss://relay.clasp.to' },
]

function selectPreset(url) {
  settings.url = url
}

function selectDiscovered(server) {
  settings.url = server.url
}

// Auto-scan on mount
onMounted(() => {
  scan()
})
</script>

<template>
  <div class="connection-panel">
    <div class="panel-header">
      <h3>Connection</h3>
      <span :class="['status-dot', { connected }]"></span>
    </div>

    <div class="field">
      <label>Server URL</label>
      <input
        v-model="settings.url"
        type="text"
        placeholder="ws://localhost:7330"
        :disabled="connected"
      />
      <div class="presets">
        <button
          v-for="preset in presets"
          :key="preset.url"
          :class="['preset-btn', { active: settings.url === preset.url }]"
          @click="selectPreset(preset.url)"
          :disabled="connected"
        >
          {{ preset.label }}
        </button>
      </div>
    </div>

    <div class="discovery-section">
      <div class="discovery-header">
        <span class="discovery-label">Discovered Servers</span>
        <button
          class="scan-btn"
          @click="scan"
          :disabled="scanning || connected"
        >
          {{ scanning ? 'Scanning...' : 'Scan' }}
        </button>
      </div>
      <div v-if="discoveredServers.length" class="discovered-list">
        <button
          v-for="server in discoveredServers"
          :key="server.url"
          :class="['discovered-item', { active: settings.url === server.url }]"
          @click="selectDiscovered(server)"
          :disabled="connected"
        >
          <span class="server-icon">●</span>
          <span class="server-name">{{ server.name }}</span>
          <span class="server-port">:{{ server.port }}</span>
        </button>
      </div>
      <div v-else-if="!scanning" class="no-servers">
        No servers found. Start a local server or use Public Relay.
      </div>
    </div>

    <div class="field">
      <label>Client Name</label>
      <input
        v-model="settings.name"
        type="text"
        placeholder="My Client"
        :disabled="connected"
      />
    </div>

    <div class="field">
      <label>Token (optional)</label>
      <input
        v-model="settings.token"
        type="password"
        placeholder="JWT token"
        :disabled="connected"
      />
    </div>

    <div v-if="error" class="error-msg">
      {{ error }}
    </div>

    <div v-if="sessionId" class="session-info">
      <span class="label">Session:</span>
      <code>{{ sessionId }}</code>
    </div>

    <button
      :class="['connect-btn', { connected }]"
      @click="connected ? disconnect() : connect()"
      :disabled="connecting"
    >
      <span v-if="connecting">Connecting...</span>
      <span v-else-if="connected">Disconnect</span>
      <span v-else>Connect</span>
    </button>

    <div class="code-hint">
      <div class="code-label">Code:</div>
      <pre><code>const client = await new ClaspBuilder('{{ settings.url }}')
  .name('{{ settings.name }}'){{ settings.token ? `
  .token('${settings.token.slice(0, 8)}...')` : '' }}
  .connect();</code></pre>
    </div>
  </div>
</template>

<style scoped>
.connection-panel {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-header h3 {
  margin: 0;
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  opacity: 0.6;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: rgba(0,0,0,0.2);
  transition: background 0.2s;
}

.status-dot.connected {
  background: #4CAF50;
  box-shadow: 0 0 6px rgba(76, 175, 80, 0.5);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.field label {
  font-size: 0.75rem;
  letter-spacing: 0.1em;
  opacity: 0.7;
}

.field input {
  padding: 0.6rem 0.8rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.5);
  font-family: inherit;
  font-size: 0.9rem;
}

.field input:focus {
  outline: none;
  border-color: var(--accent);
}

.field input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.presets {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.preset-btn {
  padding: 0.3rem 0.6rem;
  font-size: 0.7rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  letter-spacing: 0.05em;
  transition: all 0.15s;
}

.preset-btn:hover:not(:disabled) {
  background: rgba(0,0,0,0.05);
}

.preset-btn.active {
  background: var(--ink);
  color: var(--paper);
  border-color: var(--ink);
}

.preset-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.error-msg {
  padding: 0.6rem 0.8rem;
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid rgba(244, 67, 54, 0.3);
  color: #c62828;
  font-size: 0.8rem;
}

.session-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
}

.session-info .label {
  opacity: 0.6;
}

.session-info code {
  background: rgba(0,0,0,0.06);
  padding: 0.2rem 0.4rem;
  font-size: 0.7rem;
  word-break: break-all;
}

.connect-btn {
  padding: 0.8rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.85rem;
  letter-spacing: 0.15em;
  cursor: pointer;
  transition: all 0.15s;
}

.connect-btn:hover:not(:disabled) {
  background: var(--accent);
}

.connect-btn.connected {
  background: transparent;
  color: var(--ink);
  border: 1px solid rgba(0,0,0,0.2);
}

.connect-btn.connected:hover {
  border-color: #c62828;
  color: #c62828;
}

.connect-btn:disabled {
  opacity: 0.5;
  cursor: wait;
}

.code-hint {
  margin-top: 0.5rem;
  border: 1px solid rgba(0,0,0,0.1);
  background: rgba(255,255,255,0.5);
}

.code-label {
  padding: 0.4rem 0.6rem;
  font-size: 0.65rem;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  opacity: 0.5;
  border-bottom: 1px solid rgba(0,0,0,0.08);
}

.code-hint pre {
  margin: 0;
  padding: 0.6rem;
  font-size: 0.7rem;
  overflow-x: auto;
  line-height: 1.5;
}

.code-hint code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.discovery-section {
  border: 1px solid rgba(0,0,0,0.1);
  padding: 0.8rem;
  background: rgba(255,255,255,0.3);
}

.discovery-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.6rem;
}

.discovery-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  opacity: 0.6;
  text-transform: uppercase;
}

.scan-btn {
  padding: 0.25rem 0.5rem;
  font-size: 0.7rem;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.2);
  cursor: pointer;
  font-family: inherit;
  letter-spacing: 0.05em;
}

.scan-btn:hover:not(:disabled) {
  background: rgba(0,0,0,0.05);
}

.scan-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.discovered-list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.discovered-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.6rem;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.08);
  cursor: pointer;
  font-family: inherit;
  font-size: 0.8rem;
  text-align: left;
  width: 100%;
  transition: all 0.15s;
}

.discovered-item:hover:not(:disabled) {
  background: rgba(0,0,0,0.06);
  border-color: rgba(0,0,0,0.15);
}

.discovered-item.active {
  background: rgba(255, 95, 31, 0.1);
  border-color: var(--accent);
}

.discovered-item:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.server-icon {
  color: #4CAF50;
  font-size: 0.6rem;
}

.server-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.server-port {
  opacity: 0.5;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
}

.no-servers {
  font-size: 0.75rem;
  opacity: 0.5;
  text-align: center;
  padding: 0.5rem;
}
</style>
