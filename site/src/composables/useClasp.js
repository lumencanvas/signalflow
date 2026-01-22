import { ref, reactive, shallowRef, readonly } from 'vue'
import { ClaspBuilder } from '@clasp-to/core'

// Shared state across all components
const client = shallowRef(null)
const connected = ref(false)
const connecting = ref(false)
const error = ref(null)
const sessionId = ref(null)
const serverName = ref(null)
const params = reactive(new Map())
const messageLog = ref([])

// Connection settings
const settings = reactive({
  url: 'ws://localhost:7330',
  name: 'Playground Client',
  token: '',
  features: ['param', 'event', 'stream', 'gesture', 'timeline'],
})

// Maximum log entries
const MAX_LOG_ENTRIES = 500

function addLogEntry(direction, type, data) {
  const entry = {
    id: Date.now() + Math.random(),
    timestamp: new Date().toISOString(),
    direction, // 'sent' or 'received'
    type,
    data,
  }
  messageLog.value.unshift(entry)
  if (messageLog.value.length > MAX_LOG_ENTRIES) {
    messageLog.value.pop()
  }
}

async function connect() {
  if (connecting.value || connected.value) return

  connecting.value = true
  error.value = null

  try {
    const builder = new ClaspBuilder(settings.url)
      .name(settings.name)
      .features(settings.features)
      .reconnect(true)

    if (settings.token) {
      builder.token(settings.token)
    }

    const c = await builder.connect()
    client.value = c
    connected.value = true
    sessionId.value = c.session

    addLogEntry('received', 'WELCOME', { session: c.session })

    // Set up event handlers
    c.onDisconnect((reason) => {
      connected.value = false
      addLogEntry('received', 'DISCONNECT', { reason })
    })

    c.onError((err) => {
      error.value = err.message
      addLogEntry('received', 'ERROR', { message: err.message })
    })

  } catch (e) {
    error.value = e.message
    addLogEntry('received', 'ERROR', { message: e.message })
  } finally {
    connecting.value = false
  }
}

function disconnect() {
  if (client.value) {
    client.value.close()
    client.value = null
    connected.value = false
    sessionId.value = null
    params.clear()
    addLogEntry('sent', 'CLOSE', {})
  }
}

function subscribe(pattern, callback) {
  if (!client.value) return () => {}

  addLogEntry('sent', 'SUBSCRIBE', { pattern })

  return client.value.on(pattern, (value, address, meta) => {
    params.set(address, value)
    addLogEntry('received', 'PUBLISH', { address, value })
    callback?.(value, address, meta)
  })
}

function set(address, value) {
  if (!client.value) return
  client.value.set(address, value)
  params.set(address, value)
  addLogEntry('sent', 'SET', { address, value })
}

function emit(address, payload) {
  if (!client.value) return
  client.value.emit(address, payload)
  addLogEntry('sent', 'PUBLISH', { address, signal: 'event', payload })
}

function stream(address, value) {
  if (!client.value) return
  client.value.stream(address, value)
  // Don't log stream messages to avoid flooding
}

async function get(address) {
  if (!client.value) return undefined
  addLogEntry('sent', 'GET', { address })
  const value = await client.value.get(address)
  addLogEntry('received', 'SNAPSHOT', { address, value })
  return value
}

function bundle(messages, options) {
  if (!client.value) return
  client.value.bundle(messages, options)
  addLogEntry('sent', 'BUNDLE', { messages, options })
}

function cached(address) {
  return params.get(address)
}

function time() {
  return client.value?.time() ?? Date.now() * 1000
}

function clearLog() {
  messageLog.value = []
}

// Discovery state
const discoveredServers = ref([])
const scanning = ref(false)

// Scan for CLASP servers on common ports
async function scan() {
  if (scanning.value) return
  scanning.value = true
  discoveredServers.value = []

  const portsToScan = [7330, 8080, 9000]
  const hosts = ['localhost', '127.0.0.1']

  const probePromises = []

  for (const host of hosts) {
    for (const port of portsToScan) {
      probePromises.push(probeServer(host, port))
    }
  }

  const results = await Promise.allSettled(probePromises)

  // Deduplicate by URL
  const seen = new Set()
  for (const result of results) {
    if (result.status === 'fulfilled' && result.value) {
      const url = result.value.url
      if (!seen.has(url)) {
        seen.add(url)
        discoveredServers.value.push(result.value)
      }
    }
  }

  scanning.value = false
}

// Probe a single server
async function probeServer(host, port) {
  const wsUrl = `ws://${host}:${port}`

  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      ws.close()
      resolve(null)
    }, 2000)

    let ws
    try {
      ws = new WebSocket(wsUrl, 'clasp')
      ws.binaryType = 'arraybuffer'

      ws.onopen = () => {
        clearTimeout(timeout)
        ws.close()
        resolve({
          url: wsUrl,
          host,
          port,
          name: `CLASP Server (${host}:${port})`,
        })
      }

      ws.onerror = () => {
        clearTimeout(timeout)
        resolve(null)
      }

      ws.onclose = () => {
        clearTimeout(timeout)
      }
    } catch (e) {
      clearTimeout(timeout)
      resolve(null)
    }
  })
}

export function useClasp() {
  return {
    // State
    client: readonly(client),
    connected: readonly(connected),
    connecting: readonly(connecting),
    error: readonly(error),
    sessionId: readonly(sessionId),
    serverName: readonly(serverName),
    params: readonly(params),
    messageLog: readonly(messageLog),
    settings,

    // Discovery
    discoveredServers: readonly(discoveredServers),
    scanning: readonly(scanning),
    scan,

    // Methods
    connect,
    disconnect,
    subscribe,
    set,
    emit,
    stream,
    get,
    bundle,
    cached,
    time,
    clearLog,
    addLogEntry,
  }
}
