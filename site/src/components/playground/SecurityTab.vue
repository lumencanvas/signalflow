<script setup>
import { ref, computed } from 'vue'
import { useClasp } from '../../composables/useClasp'
import CodeSnippet from './CodeSnippet.vue'

const { connected, set, emit } = useClasp()

// JWT Token Builder
const tokenPayload = ref({
  sub: 'playground-user',
  name: 'Playground Client',
  scopes: ['read:/**', 'write:/playground/**'],
  exp: Math.floor(Date.now() / 1000) + 3600,
})

const newScope = ref('')

const generatedToken = computed(() => {
  // Simple base64 "token" for demonstration (not a real JWT)
  const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }))
  const payload = btoa(JSON.stringify(tokenPayload.value))
  const signature = btoa('demo-signature')
  return `${header}.${payload}.${signature}`
})

const decodedPayload = computed(() => {
  return JSON.stringify(tokenPayload.value, null, 2)
})

function addScope() {
  if (newScope.value && !tokenPayload.value.scopes.includes(newScope.value)) {
    tokenPayload.value.scopes.push(newScope.value)
    newScope.value = ''
  }
}

function removeScope(index) {
  tokenPayload.value.scopes.splice(index, 1)
}

// Lock Demo
const lockAddress = ref('/playground/locked-param')
const lockValue = ref('exclusive value')
const lockHeld = ref(false)

function acquireLock() {
  if (!connected.value) return
  // In a real implementation, this would use the lock flag
  set(lockAddress.value, { value: lockValue.value, locked: true })
  lockHeld.value = true
}

function releaseLock() {
  if (!connected.value) return
  set(lockAddress.value, { value: lockValue.value, locked: false })
  lockHeld.value = false
}

// Conflict Resolution Demo
const conflictStrategy = ref('lww')
const conflictAddress = ref('/playground/conflict-demo')
const value1 = ref(50)
const value2 = ref(75)

function simulateConflict() {
  if (!connected.value) return

  // Simulate two concurrent writes
  const addr = conflictAddress.value

  // In real CLASP, the server would resolve based on strategy
  // Here we just demonstrate the concept
  setTimeout(() => set(addr, value1.value), 0)
  setTimeout(() => set(addr, value2.value), 10)
}
</script>

<template>
  <div class="security-tab">
    <div class="security-header">
      <h3>Security Features</h3>
      <p class="hint">
        Explore CLASP's security model including JWT authentication, scoped permissions, parameter
        locking, and conflict resolution strategies.
      </p>
    </div>

    <div class="security-grid">
      <!-- JWT Token Builder -->
      <div class="security-card full-width">
        <h4>JWT Token Structure</h4>
        <p class="card-hint">
          CLASP uses JWT tokens for authentication. Tokens can contain scopes that restrict read/write
          access to specific address patterns.
        </p>

        <div class="token-builder">
          <div class="token-fields">
            <div class="field">
              <label>Subject (sub)</label>
              <input v-model="tokenPayload.sub" type="text" placeholder="user-id" />
            </div>

            <div class="field">
              <label>Name</label>
              <input v-model="tokenPayload.name" type="text" placeholder="Client Name" />
            </div>

            <div class="field">
              <label>Scopes</label>
              <div class="scopes-list">
                <div
                  v-for="(scope, i) in tokenPayload.scopes"
                  :key="i"
                  class="scope-tag"
                >
                  <code>{{ scope }}</code>
                  <button @click="removeScope(i)">x</button>
                </div>
              </div>
              <div class="add-scope">
                <input
                  v-model="newScope"
                  type="text"
                  placeholder="read:/** or write:/path/*"
                  @keyup.enter="addScope"
                />
                <button @click="addScope">Add</button>
              </div>
            </div>
          </div>

          <div class="token-output">
            <div class="output-section">
              <div class="output-label">Decoded Payload</div>
              <pre>{{ decodedPayload }}</pre>
            </div>

            <div class="output-section">
              <div class="output-label">Encoded Token</div>
              <div class="token-string">{{ generatedToken }}</div>
            </div>
          </div>
        </div>

        <CodeSnippet :code="`// Connect with JWT token
const client = await new ClaspBuilder('wss://relay.clasp.to')
  .name('${tokenPayload.name}')
  .token('${generatedToken.slice(0, 50)}...')
  .connect();

// Token payload structure:
${decodedPayload}`" />
      </div>

      <!-- Scope Examples -->
      <div class="security-card">
        <h4>Scope Patterns</h4>
        <p class="card-hint">
          Scopes use glob patterns to define permitted addresses.
        </p>

        <div class="examples">
          <div class="example">
            <code>read:/**</code>
            <span>Read all addresses</span>
          </div>
          <div class="example">
            <code>write:/lights/*</code>
            <span>Write to /lights/&lt;id&gt; only</span>
          </div>
          <div class="example">
            <code>read:/sensors/**</code>
            <span>Read all sensor values</span>
          </div>
          <div class="example">
            <code>write:/user/${tokenPayload.sub}/**</code>
            <span>Write only to own namespace</span>
          </div>
        </div>

        <CodeSnippet :code="`// Scope format: action:pattern
// Actions: read, write, admin
// Patterns: glob with * and **

const scopes = [
  'read:/**',        // Read anything
  'write:/my/**',    // Write to /my/...
  'admin:/config/*'  // Admin access to /config
];`" />
      </div>

      <!-- Parameter Locking -->
      <div class="security-card">
        <h4>Parameter Locking</h4>
        <p class="card-hint">
          Lock parameters for exclusive write access. Only the lock holder can modify the value.
        </p>

        <div class="field">
          <label>Address</label>
          <input v-model="lockAddress" type="text" :disabled="!connected" />
        </div>

        <div class="field">
          <label>Value</label>
          <input v-model="lockValue" type="text" :disabled="!connected || lockHeld" />
        </div>

        <div class="lock-status" :class="{ locked: lockHeld }">
          Status: {{ lockHeld ? 'Locked (you hold the lock)' : 'Unlocked' }}
        </div>

        <div class="lock-buttons">
          <button
            @click="acquireLock"
            :disabled="!connected || lockHeld"
          >
            Acquire Lock
          </button>
          <button
            @click="releaseLock"
            :disabled="!connected || !lockHeld"
          >
            Release Lock
          </button>
        </div>

        <CodeSnippet :code="`// Acquire lock on set
client.set('${lockAddress}', value, {
  lock: true  // Request exclusive lock
});

// Release lock
client.set('${lockAddress}', value, {
  unlock: true  // Release lock
});`" />
      </div>

      <!-- Conflict Resolution -->
      <div class="security-card">
        <h4>Conflict Resolution</h4>
        <p class="card-hint">
          CLASP supports multiple strategies for resolving concurrent write conflicts.
        </p>

        <div class="strategies">
          <div
            v-for="s in ['lww', 'max', 'min', 'lock', 'merge']"
            :key="s"
            :class="['strategy', { active: conflictStrategy === s }]"
            @click="conflictStrategy = s"
          >
            <div class="strategy-name">{{ s.toUpperCase() }}</div>
            <div class="strategy-desc">
              {{ {
                lww: 'Last Write Wins (timestamp)',
                max: 'Keep highest value',
                min: 'Keep lowest value',
                lock: 'Exclusive access required',
                merge: 'CRDT-style merge'
              }[s] }}
            </div>
          </div>
        </div>

        <div class="conflict-demo">
          <div class="field-row">
            <div class="field">
              <label>Value A</label>
              <input type="number" v-model.number="value1" :disabled="!connected" />
            </div>
            <div class="field">
              <label>Value B</label>
              <input type="number" v-model.number="value2" :disabled="!connected" />
            </div>
          </div>
          <button @click="simulateConflict" :disabled="!connected">
            Simulate Concurrent Writes
          </button>
        </div>

        <CodeSnippet :code="`// Server-side strategy configuration
// Strategy: ${conflictStrategy.toUpperCase()}

// With ${conflictStrategy}:
// A writes: ${value1}
// B writes: ${value2}
// Result: ${conflictStrategy === 'max' ? Math.max(value1, value2) :
  conflictStrategy === 'min' ? Math.min(value1, value2) :
  value2 // lww - last wins
}`" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.security-tab {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.security-header h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
  letter-spacing: 0.15em;
}

.security-header .hint {
  margin: 0;
  font-size: 0.85rem;
  opacity: 0.6;
  line-height: 1.5;
}

.security-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 1rem;
}

.security-card {
  border: 1px solid rgba(0,0,0,0.12);
  padding: 1.2rem;
  background: rgba(255,255,255,0.4);
}

.security-card.full-width {
  grid-column: 1 / -1;
}

.security-card h4 {
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
  gap: 1rem;
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

input:disabled {
  opacity: 0.5;
}

/* Token Builder */
.token-builder {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
  margin-bottom: 1rem;
}

.scopes-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin-bottom: 0.5rem;
}

.scope-tag {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.3rem 0.5rem;
  background: rgba(255, 95, 31, 0.1);
  border: 1px solid rgba(255, 95, 31, 0.3);
}

.scope-tag code {
  font-size: 0.75rem;
}

.scope-tag button {
  padding: 0 0.3rem;
  background: transparent;
  border: none;
  cursor: pointer;
  opacity: 0.6;
}

.scope-tag button:hover {
  opacity: 1;
  color: #c62828;
}

.add-scope {
  display: flex;
  gap: 0.5rem;
}

.add-scope input {
  flex: 1;
}

.add-scope button {
  padding: 0.5rem 0.8rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.add-scope button:hover {
  background: var(--accent);
}

.token-output {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.output-section {
  background: rgba(0,0,0,0.03);
  padding: 0.8rem;
}

.output-label {
  font-size: 0.7rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.5;
  margin-bottom: 0.5rem;
}

.output-section pre {
  margin: 0;
  font-size: 0.75rem;
  line-height: 1.5;
  overflow-x: auto;
}

.token-string {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.7rem;
  word-break: break-all;
  line-height: 1.4;
}

/* Examples */
.examples {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.example {
  display: flex;
  align-items: center;
  gap: 0.8rem;
  padding: 0.5rem 0.6rem;
  background: rgba(0,0,0,0.03);
}

.example code {
  font-size: 0.75rem;
  color: var(--accent);
  flex-shrink: 0;
}

.example span {
  font-size: 0.8rem;
  opacity: 0.7;
}

/* Lock Demo */
.lock-status {
  padding: 0.5rem 0.8rem;
  background: rgba(0,0,0,0.05);
  font-size: 0.85rem;
  margin-bottom: 0.8rem;
}

.lock-status.locked {
  background: rgba(76, 175, 80, 0.1);
  color: #2e7d32;
}

.lock-buttons {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.lock-buttons button {
  flex: 1;
  padding: 0.5rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.lock-buttons button:hover:not(:disabled) {
  background: var(--accent);
}

.lock-buttons button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Strategies */
.strategies {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.strategy {
  padding: 0.6rem;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.1);
  cursor: pointer;
  transition: all 0.15s;
}

.strategy:hover {
  border-color: rgba(0,0,0,0.2);
}

.strategy.active {
  background: rgba(255, 95, 31, 0.1);
  border-color: var(--accent);
}

.strategy-name {
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.1em;
  margin-bottom: 0.2rem;
}

.strategy-desc {
  font-size: 0.7rem;
  opacity: 0.6;
  line-height: 1.3;
}

.conflict-demo {
  margin-bottom: 1rem;
}

.conflict-demo button {
  width: 100%;
  padding: 0.6rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.8rem;
  cursor: pointer;
  margin-top: 0.5rem;
}

.conflict-demo button:hover:not(:disabled) {
  background: var(--accent);
}

.conflict-demo button:disabled {
  opacity: 0.4;
}

@media (max-width: 800px) {
  .token-builder {
    grid-template-columns: 1fr;
  }
}
</style>
