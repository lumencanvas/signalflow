<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useClasp } from '../../composables/useClasp'
import CodeSnippet from './CodeSnippet.vue'

const { connected, subscribe, stream, emit } = useClasp()

// Mode: 'send' or 'receive'
const mode = ref('send')

// === SEND MODE STATE ===
// Accelerometer state
const accelX = ref(0)
const accelY = ref(0)
const accelZ = ref(9.8)
const accelStreaming = ref(false)
let accelInterval = null

// Sliders
const sliders = ref([0, 0, 0, 0, 0, 0, 0, 0])

// XY Pad
const padX = ref(0.5)
const padY = ref(0.5)
const padActive = ref(false)
const padRef = ref(null)

// Stream rate
const streamRate = ref(30)

// === RECEIVE MODE STATE ===
const receivedAccel = ref({ x: 0, y: 0, z: 9.8 })
const receivedSliders = ref([0, 0, 0, 0, 0, 0, 0, 0])
const receivedPad = ref({ x: 0.5, y: 0.5 })
const receivedGyro = ref({ alpha: 0, beta: 0, gamma: 0 })
const lastUpdate = ref({})
const subscriptions = ref([])
const isSubscribed = ref(false)

// === SEND MODE FUNCTIONS ===
function startAccelStream() {
  if (!connected.value || accelStreaming.value) return
  accelStreaming.value = true

  const interval = Math.round(1000 / streamRate.value)
  accelInterval = setInterval(() => {
    stream('/sensors/accelerometer', {
      x: accelX.value,
      y: accelY.value,
      z: accelZ.value,
    })
  }, interval)
}

function stopAccelStream() {
  accelStreaming.value = false
  if (accelInterval) {
    clearInterval(accelInterval)
    accelInterval = null
  }
}

function onAccelDrag(e) {
  if (!accelStreaming.value) return

  const rect = e.currentTarget.getBoundingClientRect()
  const x = ((e.clientX - rect.left) / rect.width - 0.5) * 20
  const y = -((e.clientY - rect.top) / rect.height - 0.5) * 20

  accelX.value = Math.round(x * 100) / 100
  accelY.value = Math.round(y * 100) / 100
}

function onSliderChange(index) {
  if (!connected.value) return
  stream(`/sensors/slider/${index}`, sliders.value[index])
}

function onPadStart(e) {
  if (!connected.value) return
  padActive.value = true
  updatePadPosition(e)
}

function onPadMove(e) {
  if (!padActive.value || !connected.value) return
  updatePadPosition(e)
}

function onPadEnd() {
  if (!padActive.value) return
  padActive.value = false
  emit('/sensors/pad/release', { x: padX.value, y: padY.value })
}

function updatePadPosition(e) {
  const rect = padRef.value.getBoundingClientRect()
  const clientX = e.touches ? e.touches[0].clientX : e.clientX
  const clientY = e.touches ? e.touches[0].clientY : e.clientY

  padX.value = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
  padY.value = Math.max(0, Math.min(1, 1 - (clientY - rect.top) / rect.height))

  stream('/sensors/pad', { x: padX.value, y: padY.value })
}

// === RECEIVE MODE FUNCTIONS ===
function startReceiving() {
  if (!connected.value || isSubscribed.value) return

  // Subscribe to accelerometer
  const unsubAccel = subscribe('/sensors/accelerometer', (data) => {
    if (data && typeof data === 'object') {
      receivedAccel.value = { ...receivedAccel.value, ...data }
      lastUpdate.value.accelerometer = Date.now()
    }
  })
  subscriptions.value.push(unsubAccel)

  // Subscribe to all sliders
  for (let i = 0; i < 8; i++) {
    const unsubSlider = subscribe(`/sensors/slider/${i}`, (value) => {
      if (typeof value === 'number') {
        receivedSliders.value[i] = value
        lastUpdate.value[`slider${i}`] = Date.now()
      }
    })
    subscriptions.value.push(unsubSlider)
  }

  // Subscribe to XY pad
  const unsubPad = subscribe('/sensors/pad', (data) => {
    if (data && typeof data === 'object') {
      receivedPad.value = { ...receivedPad.value, ...data }
      lastUpdate.value.pad = Date.now()
    }
  })
  subscriptions.value.push(unsubPad)

  // Subscribe to gyroscope
  const unsubGyro = subscribe('/sensors/gyroscope', (data) => {
    if (data && typeof data === 'object') {
      receivedGyro.value = { ...receivedGyro.value, ...data }
      lastUpdate.value.gyroscope = Date.now()
    }
  })
  subscriptions.value.push(unsubGyro)

  isSubscribed.value = true
}

function stopReceiving() {
  subscriptions.value.forEach(unsub => unsub?.())
  subscriptions.value = []
  isSubscribed.value = false
}

function isRecent(key) {
  const ts = lastUpdate.value[key]
  return ts && Date.now() - ts < 500
}

// Cleanup
onUnmounted(() => {
  stopAccelStream()
  stopReceiving()
})
</script>

<template>
  <div class="sensors-tab">
    <div class="sensors-header">
      <h3>Sensor Data</h3>
      <p class="hint">
        Stream sensor data via CLASP or receive data from other connected devices.
      </p>
    </div>

    <!-- Mode Toggle -->
    <div class="mode-toggle">
      <button
        :class="['mode-btn', { active: mode === 'send' }]"
        @click="mode = 'send'; stopReceiving()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="22" y1="2" x2="11" y2="13"/>
          <polygon points="22 2 15 22 11 13 2 9 22 2"/>
        </svg>
        Send
      </button>
      <button
        :class="['mode-btn', { active: mode === 'receive' }]"
        @click="mode = 'receive'; stopAccelStream()"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/>
          <path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>
        </svg>
        Receive
      </button>
    </div>

    <!-- SEND MODE -->
    <template v-if="mode === 'send'">
      <div class="stream-rate">
        <label>Stream Rate:</label>
        <select v-model="streamRate" :disabled="accelStreaming">
          <option :value="10">10 Hz</option>
          <option :value="30">30 Hz</option>
          <option :value="60">60 Hz</option>
        </select>
      </div>

      <div class="sensors-grid">
        <!-- Accelerometer -->
        <div class="sensor-card">
          <div class="card-header">
            <h4>Accelerometer</h4>
            <button
              :class="['stream-btn', { active: accelStreaming }]"
              @click="accelStreaming ? stopAccelStream() : startAccelStream()"
              :disabled="!connected"
            >
              {{ accelStreaming ? 'Stop' : 'Start' }} Stream
            </button>
          </div>

          <div
            class="accel-pad"
            @mousemove="onAccelDrag"
            @touchmove.prevent="onAccelDrag"
          >
            <div class="accel-grid">
              <div class="axis-label x">X</div>
              <div class="axis-label y">Y</div>
            </div>
            <div
              class="accel-dot"
              :style="{
                left: `${(accelX / 20 + 0.5) * 100}%`,
                top: `${(-accelY / 20 + 0.5) * 100}%`,
              }"
            ></div>
          </div>

          <div class="accel-values">
            <div class="value-item">
              <span class="label">X:</span>
              <span class="value">{{ accelX.toFixed(2) }}</span>
            </div>
            <div class="value-item">
              <span class="label">Y:</span>
              <span class="value">{{ accelY.toFixed(2) }}</span>
            </div>
            <div class="value-item">
              <span class="label">Z:</span>
              <input
                type="number"
                v-model.number="accelZ"
                step="0.1"
                min="-20"
                max="20"
              />
            </div>
          </div>

          <CodeSnippet :code="`// Stream accelerometer data
client.stream('/sensors/accelerometer', {
  x: ${accelX.toFixed(2)},
  y: ${accelY.toFixed(2)},
  z: ${accelZ.toFixed(2)}
});`" />
        </div>

        <!-- Sliders -->
        <div class="sensor-card">
          <div class="card-header">
            <h4>Fader Bank</h4>
          </div>

          <div class="slider-bank">
            <div
              v-for="(val, i) in sliders"
              :key="i"
              class="slider-channel"
            >
              <input
                type="range"
                orient="vertical"
                min="0"
                max="1"
                step="0.01"
                v-model.number="sliders[i]"
                @input="onSliderChange(i)"
                :disabled="!connected"
              />
              <span class="slider-value">{{ sliders[i].toFixed(2) }}</span>
              <span class="slider-label">{{ i + 1 }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Stream slider value
client.stream('/sensors/slider/0', ${sliders[0].toFixed(2)});`" />
        </div>

        <!-- XY Pad -->
        <div class="sensor-card">
          <div class="card-header">
            <h4>XY Pad</h4>
          </div>

          <div
            ref="padRef"
            class="xy-pad"
            @mousedown="onPadStart"
            @mousemove="onPadMove"
            @mouseup="onPadEnd"
            @mouseleave="onPadEnd"
            @touchstart.prevent="onPadStart"
            @touchmove.prevent="onPadMove"
            @touchend="onPadEnd"
          >
            <div
              class="pad-cursor"
              :class="{ active: padActive }"
              :style="{
                left: `${padX * 100}%`,
                bottom: `${padY * 100}%`,
              }"
            ></div>
          </div>

          <div class="pad-values">
            <div class="value-item">
              <span class="label">X:</span>
              <span class="value">{{ padX.toFixed(3) }}</span>
            </div>
            <div class="value-item">
              <span class="label">Y:</span>
              <span class="value">{{ padY.toFixed(3) }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Stream XY position
client.stream('/sensors/pad', {
  x: ${padX.toFixed(3)},
  y: ${padY.toFixed(3)}
});

// Emit release event
client.emit('/sensors/pad/release', { x, y });`" />
        </div>
      </div>
    </template>

    <!-- RECEIVE MODE -->
    <template v-else>
      <div class="receive-controls">
        <button
          :class="['subscribe-btn', { active: isSubscribed }]"
          @click="isSubscribed ? stopReceiving() : startReceiving()"
          :disabled="!connected"
        >
          <svg v-if="!isSubscribed" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M5 12h14"/>
            <path d="M12 5l7 7-7 7"/>
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="6" y="4" width="4" height="16"/>
            <rect x="14" y="4" width="4" height="16"/>
          </svg>
          {{ isSubscribed ? 'Stop Listening' : 'Start Listening' }}
        </button>
        <span v-if="isSubscribed" class="listening-indicator">
          <span class="pulse"></span>
          Listening for sensor data...
        </span>
      </div>

      <div class="sensors-grid">
        <!-- Received Accelerometer -->
        <div class="sensor-card receive-card">
          <div class="card-header">
            <h4>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="12" cy="12" r="10"/>
                <path d="M12 6v6l4 2"/>
              </svg>
              Accelerometer
            </h4>
            <span :class="['status-dot', { active: isRecent('accelerometer') }]"></span>
          </div>

          <div class="accel-pad receive-visual">
            <div class="accel-grid">
              <div class="axis-label x">X</div>
              <div class="axis-label y">Y</div>
            </div>
            <div
              class="accel-dot"
              :class="{ pulse: isRecent('accelerometer') }"
              :style="{
                left: `${(receivedAccel.x / 20 + 0.5) * 100}%`,
                top: `${(-receivedAccel.y / 20 + 0.5) * 100}%`,
              }"
            ></div>
          </div>

          <div class="accel-values">
            <div :class="['value-item', { highlight: isRecent('accelerometer') }]">
              <span class="label">X:</span>
              <span class="value">{{ receivedAccel.x.toFixed(2) }}</span>
            </div>
            <div :class="['value-item', { highlight: isRecent('accelerometer') }]">
              <span class="label">Y:</span>
              <span class="value">{{ receivedAccel.y.toFixed(2) }}</span>
            </div>
            <div :class="['value-item', { highlight: isRecent('accelerometer') }]">
              <span class="label">Z:</span>
              <span class="value">{{ receivedAccel.z.toFixed(2) }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Subscribe to accelerometer
client.on('/sensors/accelerometer', (data) => {
  console.log('Accel:', data.x, data.y, data.z);
});`" />
        </div>

        <!-- Received Sliders -->
        <div class="sensor-card receive-card">
          <div class="card-header">
            <h4>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <line x1="4" y1="21" x2="4" y2="14"/>
                <line x1="4" y1="10" x2="4" y2="3"/>
                <line x1="12" y1="21" x2="12" y2="12"/>
                <line x1="12" y1="8" x2="12" y2="3"/>
                <line x1="20" y1="21" x2="20" y2="16"/>
                <line x1="20" y1="12" x2="20" y2="3"/>
                <line x1="1" y1="14" x2="7" y2="14"/>
                <line x1="9" y1="8" x2="15" y2="8"/>
                <line x1="17" y1="16" x2="23" y2="16"/>
              </svg>
              Fader Bank
            </h4>
          </div>

          <div class="slider-bank receive-visual">
            <div
              v-for="(val, i) in receivedSliders"
              :key="i"
              class="slider-channel"
            >
              <div class="slider-track">
                <div
                  class="slider-fill"
                  :class="{ pulse: isRecent(`slider${i}`) }"
                  :style="{ height: `${val * 100}%` }"
                ></div>
              </div>
              <span :class="['slider-value', { highlight: isRecent(`slider${i}`) }]">{{ val.toFixed(2) }}</span>
              <span class="slider-label">{{ i + 1 }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Subscribe to slider values
client.on('/sensors/slider/*', (value, address) => {
  const index = address.split('/').pop();
  console.log('Slider', index, '=', value);
});`" />
        </div>

        <!-- Received XY Pad -->
        <div class="sensor-card receive-card">
          <div class="card-header">
            <h4>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                <line x1="3" y1="9" x2="21" y2="9"/>
                <line x1="9" y1="21" x2="9" y2="9"/>
              </svg>
              XY Pad
            </h4>
            <span :class="['status-dot', { active: isRecent('pad') }]"></span>
          </div>

          <div class="xy-pad receive-visual">
            <div
              class="pad-cursor"
              :class="{ pulse: isRecent('pad') }"
              :style="{
                left: `${receivedPad.x * 100}%`,
                bottom: `${receivedPad.y * 100}%`,
              }"
            ></div>
          </div>

          <div class="pad-values">
            <div :class="['value-item', { highlight: isRecent('pad') }]">
              <span class="label">X:</span>
              <span class="value">{{ receivedPad.x.toFixed(3) }}</span>
            </div>
            <div :class="['value-item', { highlight: isRecent('pad') }]">
              <span class="label">Y:</span>
              <span class="value">{{ receivedPad.y.toFixed(3) }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Subscribe to XY pad
client.on('/sensors/pad', (data) => {
  console.log('Pad X:', data.x, 'Y:', data.y);
});

// Listen for release events
client.on('/sensors/pad/release', (data) => {
  console.log('Released at:', data.x, data.y);
});`" />
        </div>

        <!-- Received Gyroscope -->
        <div class="sensor-card receive-card">
          <div class="card-header">
            <h4>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="12" cy="12" r="10"/>
                <circle cx="12" cy="12" r="6"/>
                <circle cx="12" cy="12" r="2"/>
              </svg>
              Gyroscope
            </h4>
            <span :class="['status-dot', { active: isRecent('gyroscope') }]"></span>
          </div>

          <div class="gyro-visual">
            <div class="gyro-axis">
              <div class="axis-bar alpha" :style="{ width: `${Math.abs(receivedGyro.alpha) / 360 * 100}%` }"></div>
              <span class="axis-name">Alpha</span>
              <span :class="['axis-value', { highlight: isRecent('gyroscope') }]">{{ receivedGyro.alpha.toFixed(1) }}</span>
            </div>
            <div class="gyro-axis">
              <div class="axis-bar beta" :style="{ width: `${(receivedGyro.beta + 180) / 360 * 100}%` }"></div>
              <span class="axis-name">Beta</span>
              <span :class="['axis-value', { highlight: isRecent('gyroscope') }]">{{ receivedGyro.beta.toFixed(1) }}</span>
            </div>
            <div class="gyro-axis">
              <div class="axis-bar gamma" :style="{ width: `${(receivedGyro.gamma + 90) / 180 * 100}%` }"></div>
              <span class="axis-name">Gamma</span>
              <span :class="['axis-value', { highlight: isRecent('gyroscope') }]">{{ receivedGyro.gamma.toFixed(1) }}</span>
            </div>
          </div>

          <CodeSnippet :code="`// Subscribe to gyroscope
client.on('/sensors/gyroscope', (data) => {
  console.log('Gyro:', data.alpha, data.beta, data.gamma);
});`" />
        </div>
      </div>

      <div class="receive-hint">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="16" x2="12" y2="12"/>
          <line x1="12" y1="8" x2="12.01" y2="8"/>
        </svg>
        <p>
          Open another browser tab with the Sensors page in <strong>Send</strong> mode,
          or connect a mobile device to stream real sensor data.
        </p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.sensors-tab {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.sensors-header h3 {
  margin: 0 0 0.5rem;
  font-size: 1rem;
  letter-spacing: 0.15em;
}

.sensors-header .hint {
  margin: 0;
  font-size: 0.85rem;
  opacity: 0.6;
  line-height: 1.5;
}

/* Mode Toggle */
.mode-toggle {
  display: flex;
  gap: 0;
  border: 1px solid rgba(0,0,0,0.15);
  width: fit-content;
}

.mode-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1.2rem;
  background: transparent;
  border: none;
  font-family: inherit;
  font-size: 0.85rem;
  letter-spacing: 0.08em;
  cursor: pointer;
  transition: all 0.15s;
}

.mode-btn svg {
  width: 16px;
  height: 16px;
}

.mode-btn:hover {
  background: rgba(0,0,0,0.05);
}

.mode-btn.active {
  background: var(--ink);
  color: var(--paper);
}

.mode-btn:first-child {
  border-right: 1px solid rgba(0,0,0,0.15);
}

.stream-rate {
  display: flex;
  align-items: center;
  gap: 0.8rem;
}

.stream-rate label {
  font-size: 0.8rem;
  opacity: 0.7;
}

.stream-rate select {
  padding: 0.4rem 0.6rem;
  border: 1px solid rgba(0,0,0,0.15);
  background: rgba(255,255,255,0.5);
  font-family: inherit;
  font-size: 0.85rem;
}

.sensors-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1rem;
}

.sensor-card {
  border: 1px solid rgba(0,0,0,0.12);
  padding: 1.2rem;
  background: rgba(255,255,255,0.4);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
}

.card-header h4 {
  margin: 0;
  font-size: 0.85rem;
  letter-spacing: 0.12em;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.card-header h4 svg {
  width: 16px;
  height: 16px;
  opacity: 0.5;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(0,0,0,0.15);
  transition: all 0.2s;
}

.status-dot.active {
  background: #4CAF50;
  box-shadow: 0 0 8px rgba(76, 175, 80, 0.6);
}

.stream-btn {
  padding: 0.4rem 0.8rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.75rem;
  letter-spacing: 0.08em;
  cursor: pointer;
}

.stream-btn:hover:not(:disabled) {
  background: var(--accent);
}

.stream-btn.active {
  background: #4CAF50;
}

.stream-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Accelerometer */
.accel-pad {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.1);
  cursor: crosshair;
  margin-bottom: 1rem;
}

.accel-grid {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.accel-grid::before,
.accel-grid::after {
  content: '';
  position: absolute;
  background: rgba(0,0,0,0.1);
}

.accel-grid::before {
  width: 1px;
  height: 100%;
}

.accel-grid::after {
  width: 100%;
  height: 1px;
}

.axis-label {
  position: absolute;
  font-size: 0.7rem;
  opacity: 0.4;
}

.axis-label.x {
  right: 0.5rem;
  top: 50%;
  transform: translateY(-50%);
}

.axis-label.y {
  left: 50%;
  top: 0.5rem;
  transform: translateX(-50%);
}

.accel-dot {
  position: absolute;
  width: 20px;
  height: 20px;
  background: var(--accent);
  border-radius: 50%;
  transform: translate(-50%, -50%);
  box-shadow: 0 2px 8px rgba(255, 95, 31, 0.4);
  transition: all 0.1s ease-out;
}

.accel-dot.pulse {
  animation: dotPulse 0.3s ease-out;
}

@keyframes dotPulse {
  0% { transform: translate(-50%, -50%) scale(1.3); }
  100% { transform: translate(-50%, -50%) scale(1); }
}

.accel-values, .pad-values {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
}

.value-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
  transition: all 0.15s;
}

.value-item.highlight {
  color: var(--accent);
}

.value-item .label {
  opacity: 0.6;
}

.value-item .value {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.value-item input[type="number"] {
  width: 60px;
  padding: 0.3rem;
  border: 1px solid rgba(0,0,0,0.15);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.8rem;
}

/* Sliders */
.slider-bank {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 1rem;
  padding: 1rem 0.5rem;
  background: rgba(0,0,0,0.03);
}

.slider-channel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.4rem;
}

.slider-channel input[type="range"] {
  writing-mode: vertical-lr;
  direction: rtl;
  height: 120px;
  width: 24px;
  appearance: none;
  background: rgba(0,0,0,0.1);
  cursor: pointer;
}

.slider-channel input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 24px;
  height: 12px;
  background: var(--ink);
  cursor: grab;
}

.slider-channel input[type="range"]::-webkit-slider-thumb:active {
  cursor: grabbing;
  background: var(--accent);
}

/* Receive mode slider visualization */
.slider-track {
  width: 24px;
  height: 120px;
  background: rgba(0,0,0,0.1);
  position: relative;
}

.slider-fill {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: var(--accent);
  transition: height 0.1s ease-out;
}

.slider-fill.pulse {
  animation: fillPulse 0.2s ease-out;
}

@keyframes fillPulse {
  0% { opacity: 1; }
  50% { opacity: 0.6; }
  100% { opacity: 1; }
}

.slider-value {
  font-size: 0.65rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  opacity: 0.7;
  transition: all 0.15s;
}

.slider-value.highlight {
  color: var(--accent);
  opacity: 1;
}

.slider-label {
  font-size: 0.7rem;
  opacity: 0.5;
}

/* XY Pad */
.xy-pad {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.1);
  cursor: crosshair;
  margin-bottom: 1rem;
  touch-action: none;
}

.pad-cursor {
  position: absolute;
  width: 24px;
  height: 24px;
  background: var(--ink);
  border-radius: 50%;
  transform: translate(-50%, 50%);
  transition: all 0.1s ease-out;
}

.pad-cursor.active {
  background: var(--accent);
  transform: translate(-50%, 50%) scale(1.2);
}

.pad-cursor.pulse {
  animation: dotPulse 0.3s ease-out;
  background: var(--accent);
}

/* Receive Mode */
.receive-controls {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.subscribe-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1.2rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.85rem;
  letter-spacing: 0.08em;
  cursor: pointer;
  transition: all 0.15s;
}

.subscribe-btn svg {
  width: 16px;
  height: 16px;
}

.subscribe-btn:hover:not(:disabled) {
  background: var(--accent);
}

.subscribe-btn.active {
  background: #4CAF50;
}

.subscribe-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.listening-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8rem;
  opacity: 0.7;
}

.pulse {
  width: 8px;
  height: 8px;
  background: #4CAF50;
  border-radius: 50%;
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.2); }
}

.receive-card {
  border-color: rgba(76, 175, 80, 0.2);
}

.receive-visual {
  cursor: default;
}

/* Gyroscope */
.gyro-visual {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  margin-bottom: 1rem;
  padding: 1rem;
  background: rgba(0,0,0,0.03);
}

.gyro-axis {
  display: flex;
  align-items: center;
  gap: 0.8rem;
  position: relative;
  height: 24px;
  background: rgba(0,0,0,0.05);
}

.axis-bar {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  transition: width 0.1s ease-out;
}

.axis-bar.alpha { background: #FF5F1F; }
.axis-bar.beta { background: #2196F3; }
.axis-bar.gamma { background: #4CAF50; }

.axis-name {
  position: relative;
  z-index: 1;
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  padding-left: 0.5rem;
  min-width: 50px;
}

.axis-value {
  position: relative;
  z-index: 1;
  margin-left: auto;
  padding-right: 0.5rem;
  font-size: 0.75rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  transition: all 0.15s;
}

.axis-value.highlight {
  color: var(--accent);
  font-weight: 600;
}

.receive-hint {
  display: flex;
  align-items: flex-start;
  gap: 0.8rem;
  padding: 1rem;
  background: rgba(0,0,0,0.03);
  border: 1px solid rgba(0,0,0,0.08);
}

.receive-hint svg {
  width: 20px;
  height: 20px;
  opacity: 0.4;
  flex-shrink: 0;
  margin-top: 0.1rem;
}

.receive-hint p {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.5;
  opacity: 0.7;
}

.receive-hint strong {
  color: var(--accent);
}
</style>
