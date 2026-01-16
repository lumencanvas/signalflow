<script setup>
import { ref, computed, onUnmounted } from 'vue'
import { useClasp } from '../../composables/useClasp'
import CodeSnippet from './CodeSnippet.vue'

const { connected, params, subscribe, set, emit, get } = useClasp()

const subscriptions = ref([])
const subscribePattern = ref('/playground/**')
const setAddress = ref('/playground/test')
const setValue = ref('Hello CLASP!')
const valueType = ref('string')
const emitAddress = ref('/playground/events/click')
const emitPayload = ref('{ "x": 100, "y": 200 }')
const getAddress = ref('/playground/test')
const getResult = ref(null)

const activeSubscriptions = ref([])

const sortedParams = computed(() => {
  const entries = Array.from(params.entries())
  return entries.sort((a, b) => a[0].localeCompare(b[0]))
})

function parseValue(val, type) {
  switch (type) {
    case 'number':
      return parseFloat(val) || 0
    case 'boolean':
      return val === 'true' || val === '1'
    case 'json':
      try {
        return JSON.parse(val)
      } catch {
        return val
      }
    default:
      return val
  }
}

function formatValue(val) {
  if (val === null || val === undefined) return 'null'
  if (typeof val === 'object') return JSON.stringify(val, null, 2)
  return String(val)
}

function doSubscribe() {
  if (!connected.value || !subscribePattern.value) return

  const unsub = subscribe(subscribePattern.value, (value, address) => {
    // Values are automatically stored in params
  })

  activeSubscriptions.value.push({
    pattern: subscribePattern.value,
    unsub,
  })
}

function unsubscribe(index) {
  const sub = activeSubscriptions.value[index]
  if (sub) {
    sub.unsub()
    activeSubscriptions.value.splice(index, 1)
  }
}

function doSet() {
  if (!connected.value) return
  const value = parseValue(setValue.value, valueType.value)
  set(setAddress.value, value)
}

function doEmit() {
  if (!connected.value) return
  let payload
  try {
    payload = JSON.parse(emitPayload.value)
  } catch {
    payload = emitPayload.value
  }
  emit(emitAddress.value, payload)
}

async function doGet() {
  if (!connected.value) return
  getResult.value = await get(getAddress.value)
}

// Cleanup on unmount
onUnmounted(() => {
  activeSubscriptions.value.forEach(sub => sub.unsub())
})
</script>

<template>
  <div class="explorer-tab">
    <div class="explorer-grid">
      <!-- Subscribe Section -->
      <div class="explorer-card">
        <h3>Subscribe</h3>
        <p class="hint">Listen to address patterns using wildcards (* for single, ** for multi-level)</p>

        <div class="input-row">
          <input
            v-model="subscribePattern"
            type="text"
            placeholder="/path/to/*"
            :disabled="!connected"
          />
          <button @click="doSubscribe" :disabled="!connected">Subscribe</button>
        </div>

        <div v-if="activeSubscriptions.length" class="active-subs">
          <div class="sub-label">Active Subscriptions:</div>
          <div
            v-for="(sub, i) in activeSubscriptions"
            :key="i"
            class="sub-item"
          >
            <code>{{ sub.pattern }}</code>
            <button class="unsub-btn" @click="unsubscribe(i)">x</button>
          </div>
        </div>

        <CodeSnippet :code="`client.on('${subscribePattern}', (value, address) => {
  console.log(address, value);
});`" />
      </div>

      <!-- Set Section -->
      <div class="explorer-card">
        <h3>Set Parameter</h3>
        <p class="hint">Set a parameter value at an address</p>

        <div class="field">
          <label>Address</label>
          <input
            v-model="setAddress"
            type="text"
            placeholder="/path/to/param"
            :disabled="!connected"
          />
        </div>

        <div class="field-row">
          <div class="field" style="flex: 2">
            <label>Value</label>
            <input
              v-model="setValue"
              type="text"
              placeholder="value"
              :disabled="!connected"
            />
          </div>
          <div class="field" style="flex: 1">
            <label>Type</label>
            <select v-model="valueType" :disabled="!connected">
              <option value="string">String</option>
              <option value="number">Number</option>
              <option value="boolean">Boolean</option>
              <option value="json">JSON</option>
            </select>
          </div>
        </div>

        <button class="action-btn" @click="doSet" :disabled="!connected">Set</button>

        <CodeSnippet :code="`client.set('${setAddress}', ${valueType === 'string' ? `'${setValue}'` : setValue});`" />
      </div>

      <!-- Emit Section -->
      <div class="explorer-card">
        <h3>Emit Event</h3>
        <p class="hint">Fire a one-time event with optional payload</p>

        <div class="field">
          <label>Address</label>
          <input
            v-model="emitAddress"
            type="text"
            placeholder="/events/trigger"
            :disabled="!connected"
          />
        </div>

        <div class="field">
          <label>Payload (JSON)</label>
          <textarea
            v-model="emitPayload"
            rows="2"
            placeholder='{ "key": "value" }'
            :disabled="!connected"
          ></textarea>
        </div>

        <button class="action-btn" @click="doEmit" :disabled="!connected">Emit</button>

        <CodeSnippet :code="`client.emit('${emitAddress}', ${emitPayload});`" />
      </div>

      <!-- Get Section -->
      <div class="explorer-card">
        <h3>Get Value</h3>
        <p class="hint">Request current value from server</p>

        <div class="input-row">
          <input
            v-model="getAddress"
            type="text"
            placeholder="/path/to/param"
            :disabled="!connected"
          />
          <button @click="doGet" :disabled="!connected">Get</button>
        </div>

        <div v-if="getResult !== null" class="result">
          <div class="result-label">Result:</div>
          <pre>{{ formatValue(getResult) }}</pre>
        </div>

        <CodeSnippet :code="`const value = await client.get('${getAddress}');`" />
      </div>
    </div>

    <!-- Live Values -->
    <div class="live-values">
      <h3>Live Values</h3>
      <p v-if="!sortedParams.length" class="empty-hint">
        Subscribe to patterns to see live values here
      </p>
      <div v-else class="values-list">
        <div v-for="[address, value] in sortedParams" :key="address" class="value-row">
          <code class="address">{{ address }}</code>
          <span class="value">{{ formatValue(value) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.explorer-tab {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.explorer-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 1rem;
}

.explorer-card {
  border: 1px solid rgba(0,0,0,0.12);
  padding: 1.2rem;
  background: rgba(255,255,255,0.4);
}

.explorer-card h3 {
  margin: 0 0 0.5rem;
  font-size: 0.9rem;
  letter-spacing: 0.15em;
}

.explorer-card .hint {
  margin: 0 0 1rem;
  font-size: 0.8rem;
  opacity: 0.6;
  line-height: 1.4;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  margin-bottom: 0.8rem;
}

.field label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  opacity: 0.6;
}

.field-row {
  display: flex;
  gap: 0.8rem;
}

input, select, textarea {
  padding: 0.6rem 0.8rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.5);
  font-family: inherit;
  font-size: 0.85rem;
}

input:focus, select:focus, textarea:focus {
  outline: none;
  border-color: var(--accent);
}

input:disabled, select:disabled, textarea:disabled {
  opacity: 0.5;
}

textarea {
  resize: vertical;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8rem;
}

.input-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.8rem;
}

.input-row input {
  flex: 1;
}

.input-row button, .action-btn {
  padding: 0.6rem 1rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.8rem;
  letter-spacing: 0.1em;
  cursor: pointer;
  white-space: nowrap;
}

.input-row button:hover:not(:disabled), .action-btn:hover:not(:disabled) {
  background: var(--accent);
}

.input-row button:disabled, .action-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.action-btn {
  width: 100%;
  margin-bottom: 1rem;
}

.active-subs {
  margin-bottom: 1rem;
}

.sub-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  opacity: 0.6;
  margin-bottom: 0.4rem;
}

.sub-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.4rem 0.6rem;
  background: rgba(0,0,0,0.04);
  margin-bottom: 0.3rem;
}

.sub-item code {
  font-size: 0.75rem;
}

.unsub-btn {
  padding: 0.2rem 0.5rem;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.2);
  cursor: pointer;
  font-size: 0.7rem;
}

.unsub-btn:hover {
  background: rgba(244, 67, 54, 0.1);
  border-color: #c62828;
  color: #c62828;
}

.result {
  margin-bottom: 1rem;
  padding: 0.6rem;
  background: rgba(0,0,0,0.04);
}

.result-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  opacity: 0.6;
  margin-bottom: 0.3rem;
}

.result pre {
  margin: 0;
  font-size: 0.8rem;
  white-space: pre-wrap;
  word-break: break-all;
}

.live-values {
  border: 1px solid rgba(0,0,0,0.12);
  padding: 1.2rem;
  background: rgba(255,255,255,0.4);
}

.live-values h3 {
  margin: 0 0 1rem;
  font-size: 0.9rem;
  letter-spacing: 0.15em;
}

.empty-hint {
  margin: 0;
  opacity: 0.5;
  font-size: 0.85rem;
}

.values-list {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  max-height: 300px;
  overflow-y: auto;
}

.value-row {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  padding: 0.5rem 0.6rem;
  background: rgba(0,0,0,0.03);
}

.value-row .address {
  flex-shrink: 0;
  font-size: 0.75rem;
  color: var(--accent);
}

.value-row .value {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8rem;
  word-break: break-all;
}
</style>
