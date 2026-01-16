<script setup>
import { ref, computed } from 'vue'
import { useClasp } from '../../composables/useClasp'

const { messageLog, clearLog } = useClasp()

const filter = ref('')
const showSent = ref(true)
const showReceived = ref(true)

const filteredMessages = computed(() => {
  return messageLog.value.filter((msg) => {
    // Direction filter
    if (!showSent.value && msg.direction === 'sent') return false
    if (!showReceived.value && msg.direction === 'received') return false

    // Type filter
    if (filter.value && !msg.type.toLowerCase().includes(filter.value.toLowerCase())) {
      return false
    }

    return true
  })
})

function formatData(data) {
  if (!data || typeof data !== 'object') return String(data)
  return JSON.stringify(data, null, 2)
}

function formatTime(timestamp) {
  return new Date(timestamp).toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
  })
}

function exportLog() {
  const content = messageLog.value.map((msg) => {
    return `[${formatTime(msg.timestamp)}] ${msg.direction.toUpperCase()} ${msg.type}: ${JSON.stringify(msg.data)}`
  }).join('\n')

  const blob = new Blob([content], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `clasp-log-${Date.now()}.txt`
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <div class="console-panel">
    <div class="console-toolbar">
      <div class="toolbar-left">
        <input
          v-model="filter"
          type="text"
          placeholder="Filter by type..."
          class="filter-input"
        />

        <label class="checkbox-label">
          <input type="checkbox" v-model="showSent" />
          <span class="sent-badge">Sent</span>
        </label>

        <label class="checkbox-label">
          <input type="checkbox" v-model="showReceived" />
          <span class="received-badge">Received</span>
        </label>
      </div>

      <div class="toolbar-right">
        <span class="message-count">{{ filteredMessages.length }} messages</span>
        <button @click="exportLog" class="toolbar-btn" :disabled="!messageLog.length">
          Export
        </button>
        <button @click="clearLog" class="toolbar-btn" :disabled="!messageLog.length">
          Clear
        </button>
      </div>
    </div>

    <div class="console-messages">
      <div v-if="!filteredMessages.length" class="empty-console">
        No messages yet. Connect to a server and interact to see the message log.
      </div>

      <div
        v-for="msg in filteredMessages"
        :key="msg.id"
        :class="['message-entry', msg.direction]"
      >
        <div class="message-meta">
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
          <span :class="['message-direction', msg.direction]">
            {{ msg.direction === 'sent' ? '→' : '←' }}
          </span>
          <span class="message-type">{{ msg.type }}</span>
        </div>
        <pre class="message-data">{{ formatData(msg.data) }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.console-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: rgba(255,255,255,0.3);
}

.console-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid rgba(0,0,0,0.1);
  background: rgba(0,0,0,0.02);
  gap: 1rem;
  flex-wrap: wrap;
}

.toolbar-left, .toolbar-right {
  display: flex;
  align-items: center;
  gap: 0.8rem;
}

.filter-input {
  padding: 0.3rem 0.6rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.5);
  font-family: inherit;
  font-size: 0.8rem;
  width: 150px;
}

.filter-input:focus {
  outline: none;
  border-color: var(--accent);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  font-size: 0.75rem;
  cursor: pointer;
}

.checkbox-label input {
  cursor: pointer;
}

.sent-badge, .received-badge {
  padding: 0.15rem 0.4rem;
  font-size: 0.7rem;
  letter-spacing: 0.05em;
}

.sent-badge {
  background: rgba(76, 175, 80, 0.15);
  color: #2e7d32;
}

.received-badge {
  background: rgba(33, 150, 243, 0.15);
  color: #1565c0;
}

.message-count {
  font-size: 0.75rem;
  opacity: 0.6;
}

.toolbar-btn {
  padding: 0.3rem 0.6rem;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.15);
  font-family: inherit;
  font-size: 0.75rem;
  cursor: pointer;
}

.toolbar-btn:hover:not(:disabled) {
  background: rgba(0,0,0,0.05);
}

.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.console-messages {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
}

.empty-console {
  padding: 1rem;
  text-align: center;
  opacity: 0.5;
  font-family: inherit;
}

.message-entry {
  display: flex;
  flex-direction: column;
  padding: 0.4rem 0.6rem;
  margin-bottom: 0.3rem;
  border-left: 3px solid transparent;
}

.message-entry.sent {
  background: rgba(76, 175, 80, 0.05);
  border-left-color: #4CAF50;
}

.message-entry.received {
  background: rgba(33, 150, 243, 0.05);
  border-left-color: #2196F3;
}

.message-meta {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.2rem;
}

.message-time {
  opacity: 0.5;
  font-size: 0.7rem;
}

.message-direction {
  font-weight: bold;
}

.message-direction.sent {
  color: #4CAF50;
}

.message-direction.received {
  color: #2196F3;
}

.message-type {
  font-weight: 600;
  color: var(--accent);
}

.message-data {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  opacity: 0.8;
  font-size: 0.7rem;
  line-height: 1.4;
  max-height: 100px;
  overflow-y: auto;
}
</style>
