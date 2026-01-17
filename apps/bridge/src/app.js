/**
 * CLASP Bridge - Main Application v2
 * Full-featured protocol mapping and bridging
 */

// State
const state = {
  servers: [],      // User-created servers
  devices: [],      // Discovered devices
  bridges: [],
  mappings: [],
  signals: [],
  serverLogs: new Map(), // Server ID -> log entries
  signalRate: 0,
  paused: false,
  scanning: false,
  activeTab: 'bridges',
  learnMode: false,
  learnTarget: null, // 'source' or 'target'
  editingMapping: null,
  editingServer: null, // Server being edited
  monitorFilter: '',
};

// Signal rate counter (at module level for hoisting)
let signalCount = 0;

// DOM Elements cache
const $ = (id) => document.getElementById(id);
const $$ = (sel) => document.querySelectorAll(sel);

// Icons (SVG strings)
const icons = {
  play: '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>',
  pause: '<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>',
  scan: '<svg class="icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 11-6.219-8.56"/></svg>',
  delete: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>',
  arrow: '<svg class="bridge-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>',
  bridge: '<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 12h16M8 8l-4 4 4 4M16 8l4 4-4 4"/></svg>',
  mapping: '<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="6" cy="12" r="3"/><circle cx="18" cy="12" r="3"/><line x1="9" y1="12" x2="15" y2="12"/></svg>',
};

// Protocol display names
const protocolNames = {
  osc: 'OSC',
  midi: 'MIDI',
  artnet: 'Art-Net',
  dmx: 'DMX',
  clasp: 'CLASP',
  mqtt: 'MQTT',
  websocket: 'WS',
  http: 'HTTP',
};

// Default addresses for protocols
const defaultAddresses = {
  osc: '0.0.0.0:9000',
  midi: 'default',
  artnet: '0.0.0.0:6454',
  dmx: '/dev/ttyUSB0',
  clasp: 'localhost:7330',
  mqtt: 'localhost:1883',
  websocket: '0.0.0.0:8080',
  http: '0.0.0.0:3000',
};

// Initialize application
async function init() {
  console.log('CLASP Bridge v2 initializing...');

  // Load saved data from localStorage
  loadMappingsFromStorage();

  // Restore saved servers and bridges (reconnect them)
  await restoreServersOnStartup();
  await restoreBridgesOnStartup();

  // Also try to load any discovered devices from backend
  await Promise.all([loadDevices(), loadBridges()]);

  // Set up UI
  setupTabs();
  setupModals();
  setupEventListeners();
  setupProtocolFieldSwitching();
  setupServerTypeFieldSwitching();
  setupTransformParams();
  setupLearnMode();

  // Initial render
  renderServers();
  renderDevices();
  renderBridges();
  renderMappings();
  renderSignalMonitor();
  updateStatus();
  updateMappingCount();

  // Start rate counter
  setInterval(updateSignalRate, 1000);

  console.log('CLASP Bridge initialized');
}

// ============================================
// Data Loading
// ============================================

async function loadDevices() {
  try {
    if (window.clasp) {
      state.devices = await window.clasp.getDevices();
    }
  } catch (e) {
    console.error('Failed to load devices:', e);
  }
}

async function loadBridges() {
  try {
    if (window.clasp) {
      state.bridges = await window.clasp.getBridges();
    }
  } catch (e) {
    console.error('Failed to load bridges:', e);
  }
}

function loadMappingsFromStorage() {
  try {
    const saved = localStorage.getItem('clasp-mappings');
    if (saved) {
      state.mappings = JSON.parse(saved);
    }
  } catch (e) {
    console.error('Failed to load mappings:', e);
  }
}

function saveMappingsToStorage() {
  try {
    localStorage.setItem('clasp-mappings', JSON.stringify(state.mappings));
  } catch (e) {
    console.error('Failed to save mappings:', e);
  }
}

function loadServersFromStorage() {
  try {
    const saved = localStorage.getItem('clasp-servers');
    if (saved) {
      const servers = JSON.parse(saved);
      // Mark all as disconnected initially (will be reconnected)
      return servers.map(s => ({ ...s, status: 'disconnected' }));
    }
  } catch (e) {
    console.error('Failed to load servers from storage:', e);
  }
  return [];
}

function saveServersToStorage() {
  try {
    // Save server configs (not runtime status)
    const serversToSave = state.servers.map(s => ({
      id: s.id,
      type: s.type,
      protocol: s.protocol,
      name: s.name,
      address: s.address,
      // Protocol-specific configs
      bind: s.bind,
      port: s.port,
      host: s.host,
      topics: s.topics,
      mode: s.mode,
      basePath: s.basePath,
      cors: s.cors,
      subnet: s.subnet,
      universe: s.universe,
      // DMX specific
      serialPort: s.serialPort,
      // Security
      token: s.token,
    }));
    localStorage.setItem('clasp-servers', JSON.stringify(serversToSave));
  } catch (e) {
    console.error('Failed to save servers:', e);
  }
}

function loadBridgesFromStorage() {
  try {
    const saved = localStorage.getItem('clasp-bridges');
    if (saved) {
      const bridges = JSON.parse(saved);
      return bridges.map(b => ({ ...b, active: false }));
    }
  } catch (e) {
    console.error('Failed to load bridges from storage:', e);
  }
  return [];
}

function saveBridgesToStorage() {
  try {
    localStorage.setItem('clasp-bridges', JSON.stringify(state.bridges));
  } catch (e) {
    console.error('Failed to save bridges:', e);
  }
}

async function restoreServersOnStartup() {
  const savedServers = loadServersFromStorage();
  for (const serverConfig of savedServers) {
    try {
      if (window.clasp) {
        // Try to restart the server
        const result = await window.clasp.startServer(serverConfig);
        serverConfig.id = result?.id || serverConfig.id;
        serverConfig.status = 'connected';
      } else {
        serverConfig.status = 'connected'; // Mock mode
      }
      state.servers.push(serverConfig);
    } catch (err) {
      console.warn(`Failed to restore server ${serverConfig.name}:`, err);
      serverConfig.status = 'error';
      serverConfig.error = err.message;
      state.servers.push(serverConfig);
    }
  }
}

async function restoreBridgesOnStartup() {
  const savedBridges = loadBridgesFromStorage();
  for (const bridgeConfig of savedBridges) {
    try {
      if (window.clasp) {
        const bridge = await window.clasp.createBridge(bridgeConfig);
        bridgeConfig.id = bridge?.id || bridgeConfig.id;
        bridgeConfig.active = true;
      } else {
        bridgeConfig.active = true; // Mock mode
      }
      state.bridges.push(bridgeConfig);
    } catch (err) {
      console.warn(`Failed to restore bridge:`, err);
      bridgeConfig.active = false;
      state.bridges.push(bridgeConfig);
    }
  }
}

// ============================================
// Tab Management
// ============================================

function setupTabs() {
  const tabs = $$('.tab');
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const tabName = tab.dataset.tab;
      switchTab(tabName);
    });
  });
}

function switchTab(tabName) {
  state.activeTab = tabName;

  // Update tab buttons
  $$('.tab').forEach(tab => {
    tab.classList.toggle('active', tab.dataset.tab === tabName);
  });

  // Update panels
  $$('.tab-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `panel-${tabName}`);
  });
}

// ============================================
// Modal Management
// ============================================

function setupModals() {
  // Close buttons
  $$('[data-close-modal]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const modal = e.target.closest('dialog');
      modal?.close();
      resetLearnMode();
    });
  });

  // Click outside to close
  $$('.modal').forEach(modal => {
    modal.addEventListener('click', (e) => {
      if (e.target === modal) {
        modal.close();
        resetLearnMode();
      }
    });
  });
}

// ============================================
// Protocol Field Switching
// ============================================

function setupProtocolFieldSwitching() {
  // Source protocol in mapping modal
  $('mapping-source-protocol')?.addEventListener('change', (e) => {
    updateProtocolFields('source', e.target.value);
  });

  // Target protocol in mapping modal
  $('mapping-target-protocol')?.addEventListener('change', (e) => {
    updateProtocolFields('target', e.target.value);
  });

  // Source protocol in bridge modal
  $('bridge-source')?.addEventListener('change', (e) => {
    updateBridgeAddressPlaceholder('source', e.target.value);
  });

  // Target protocol in bridge modal
  $('bridge-target')?.addEventListener('change', (e) => {
    updateBridgeAddressPlaceholder('target', e.target.value);
  });
}

function updateProtocolFields(side, protocol) {
  // Hide all protocol-specific fields for this side
  const claspFields = $(`${side}-clasp-fields`);
  const oscFields = $(`${side}-osc-fields`);
  const midiFields = $(`${side}-midi-fields`);
  const dmxFields = $(`${side}-dmx-fields`);

  claspFields?.classList.add('hidden');
  oscFields?.classList.add('hidden');
  midiFields?.classList.add('hidden');
  dmxFields?.classList.add('hidden');

  // Show appropriate fields
  switch (protocol) {
    case 'clasp':
      claspFields?.classList.remove('hidden');
      break;
    case 'osc':
      oscFields?.classList.remove('hidden');
      break;
    case 'midi':
      midiFields?.classList.remove('hidden');
      break;
    case 'artnet':
    case 'dmx':
      dmxFields?.classList.remove('hidden');
      break;
  }
}

function updateBridgeAddressPlaceholder(side, protocol) {
  const input = $(`bridge-${side}-addr`);
  if (input) {
    input.placeholder = defaultAddresses[protocol] || '';
  }
}

// ============================================
// Server Type Field Switching
// ============================================

function setupServerTypeFieldSwitching() {
  $('server-type')?.addEventListener('change', (e) => {
    updateServerTypeFields(e.target.value);
  });
}

function updateServerTypeFields(serverType) {
  // Hide all server fields
  const allFields = ['clasp', 'osc', 'midi', 'mqtt', 'websocket', 'socketio', 'http', 'artnet', 'dmx'];
  allFields.forEach(type => {
    const fields = $(`server-${type}-fields`);
    if (fields) {
      fields.classList.add('hidden');
    }
  });

  // Show appropriate fields
  const targetFields = $(`server-${serverType}-fields`);
  if (targetFields) {
    targetFields.classList.remove('hidden');
  }

  // Update hint text
  const hints = {
    clasp: 'Full CLASP protocol server - other apps can connect and exchange signals',
    osc: 'Open Sound Control server - receive OSC messages from controllers and apps',
    midi: 'MIDI bridge - connect to MIDI devices and translate to/from CLASP signals',
    mqtt: 'MQTT client - connect to an MQTT broker for IoT device communication',
    websocket: 'WebSocket bridge - accept JSON messages from web apps',
    socketio: 'Socket.IO bridge - real-time bidirectional event-based communication',
    http: 'HTTP REST API - expose signals as HTTP endpoints for webhooks and integrations',
    artnet: 'Art-Net receiver - receive DMX512 data over Ethernet from lighting consoles',
    dmx: 'DMX interface - connect directly to DMX fixtures via USB adapter',
  };
  const hintEl = $('server-type-hint');
  if (hintEl) {
    hintEl.textContent = hints[serverType] || '';
  }
}

// ============================================
// Transform Parameters
// ============================================

function setupTransformParams() {
  $('mapping-transform')?.addEventListener('change', (e) => {
    updateTransformParams(e.target.value);
    updateTransformPreview();
  });

  // Value type changes
  $('source-value-type')?.addEventListener('change', (e) => {
    const valueType = e.target.value;
    const jsonPathGroup = $('source-json-path-group');
    if (jsonPathGroup) {
      jsonPathGroup.classList.toggle('hidden', valueType !== 'json' && valueType !== 'array');
    }
  });

  $('target-value-type')?.addEventListener('change', (e) => {
    const valueType = e.target.value;
    const jsonTemplateGroup = $('target-json-template-group');
    if (jsonTemplateGroup) {
      jsonTemplateGroup.classList.toggle('hidden', valueType !== 'json');
    }
  });

  // Transform input changes for preview
  const transformInputs = document.querySelectorAll('#scale-params input, #clamp-params input, #threshold-params input, [name="expression"]');
  transformInputs.forEach(input => {
    input.addEventListener('input', () => updateTransformPreview());
  });

  // JS test button
  $('test-js-btn')?.addEventListener('click', testJavaScriptTransform);
}

function updateTransformParams(transform) {
  // Hide all transform params
  $('scale-params')?.classList.add('hidden');
  $('clamp-params')?.classList.add('hidden');
  $('threshold-params')?.classList.add('hidden');
  $('expression-params')?.classList.add('hidden');
  $('javascript-params')?.classList.add('hidden');

  // Show appropriate params
  switch (transform) {
    case 'scale':
      $('scale-params')?.classList.remove('hidden');
      break;
    case 'clamp':
      $('clamp-params')?.classList.remove('hidden');
      break;
    case 'threshold':
      $('threshold-params')?.classList.remove('hidden');
      break;
    case 'expression':
      $('expression-params')?.classList.remove('hidden');
      break;
    case 'javascript':
      $('javascript-params')?.classList.remove('hidden');
      break;
  }
}

function updateTransformPreview() {
  const previewInput = $('preview-input');
  const previewOutput = $('preview-output');
  if (!previewInput || !previewOutput) return;

  const testValue = 0.5;
  const transformType = $('mapping-transform')?.value || 'direct';

  let output;
  try {
    output = applyTransformForPreview(testValue, transformType);
    previewOutput.textContent = typeof output === 'number' ? output.toFixed(3) : String(output);
    previewOutput.classList.remove('error');
  } catch (e) {
    previewOutput.textContent = 'ERR';
    previewOutput.classList.add('error');
  }
}

function applyTransformForPreview(value, transformType) {
  switch (transformType) {
    case 'direct':
      return value;
    case 'scale': {
      const inMin = parseFloat(document.querySelector('[name="scaleInMin"]')?.value) || 0;
      const inMax = parseFloat(document.querySelector('[name="scaleInMax"]')?.value) || 1;
      const outMin = parseFloat(document.querySelector('[name="scaleOutMin"]')?.value) || 0;
      const outMax = parseFloat(document.querySelector('[name="scaleOutMax"]')?.value) || 127;
      const normalized = (value - inMin) / (inMax - inMin);
      return outMin + normalized * (outMax - outMin);
    }
    case 'invert':
      return 1 - value;
    case 'clamp': {
      const min = parseFloat(document.querySelector('[name="clampMin"]')?.value) || 0;
      const max = parseFloat(document.querySelector('[name="clampMax"]')?.value) || 1;
      return Math.min(max, Math.max(min, value));
    }
    case 'round':
      return Math.round(value);
    case 'threshold': {
      const threshold = parseFloat(document.querySelector('[name="threshold"]')?.value) || 0.5;
      return value >= threshold ? 1 : 0;
    }
    case 'toggle':
      return value > 0.5 ? 1 : 0;
    case 'gate':
      return value > 0 ? 1 : 0;
    case 'trigger':
      return 1; // Simplified for preview
    case 'expression': {
      const expr = document.querySelector('[name="expression"]')?.value || 'value';
      return evaluateExpression(expr, value);
    }
    case 'javascript':
      return value; // Can't preview JS without running it
    default:
      return value;
  }
}

function evaluateExpression(expr, value) {
  // Simple expression evaluator (safe subset)
  const safeExpr = expr
    .replace(/\bvalue\b/g, String(value))
    .replace(/\bsin\b/g, 'Math.sin')
    .replace(/\bcos\b/g, 'Math.cos')
    .replace(/\btan\b/g, 'Math.tan')
    .replace(/\babs\b/g, 'Math.abs')
    .replace(/\bmin\b/g, 'Math.min')
    .replace(/\bmax\b/g, 'Math.max')
    .replace(/\bpow\b/g, 'Math.pow')
    .replace(/\bsqrt\b/g, 'Math.sqrt')
    .replace(/\bfloor\b/g, 'Math.floor')
    .replace(/\bceil\b/g, 'Math.ceil')
    .replace(/\bround\b/g, 'Math.round')
    .replace(/\bPI\b/g, 'Math.PI');

  // Basic validation - only allow safe characters
  if (!/^[0-9+\-*/%().Math\s,]+$/.test(safeExpr)) {
    throw new Error('Invalid expression');
  }

  return Function(`"use strict"; return (${safeExpr})`)();
}

function testJavaScriptTransform() {
  const resultEl = $('js-test-result');
  if (!resultEl) return;

  const code = document.querySelector('[name="javascriptCode"]')?.value || '';
  const testInput = 0.5;

  try {
    // Create a sandboxed function
    const fn = new Function('input', `
      ${code}
      return transform(input);
    `);

    const result = fn(testInput);
    resultEl.textContent = `Input: ${testInput} → Output: ${JSON.stringify(result)}`;
    resultEl.className = 'js-test-result success';
  } catch (e) {
    resultEl.textContent = `Error: ${e.message}`;
    resultEl.className = 'js-test-result error';
  }
}

// ============================================
// Learn Mode
// ============================================

function setupLearnMode() {
  // Global learn button (in the mappings toolbar)
  $('learn-btn')?.addEventListener('click', () => {
    // First open the mapping modal if not open
    const modal = $('mapping-modal');
    if (!modal?.open) {
      openMappingModal();
    }
    toggleLearnMode('source');
  });

  // Source learn button in modal (CLASP fields)
  $('learn-source-btn')?.addEventListener('click', () => {
    toggleLearnMode('source');
  });

  // OSC learn button in modal
  $('learn-source-osc-btn')?.addEventListener('click', () => {
    toggleLearnMode('source');
  });
}

function toggleLearnMode(target) {
  if (state.learnMode && state.learnTarget === target) {
    // Turn off
    resetLearnMode();
  } else {
    // Turn on
    state.learnMode = true;
    state.learnTarget = target;

    // Visual feedback - add pulsing animation to all learn buttons
    const learnButtons = [$('learn-btn'), $('learn-source-btn'), $('learn-source-osc-btn')];
    learnButtons.forEach(btn => btn?.classList.add('learn-active'));

    // Show notification that we're waiting for a signal
    showLearnNotification('Waiting for incoming signal...');
  }
}

function resetLearnMode() {
  state.learnMode = false;
  state.learnTarget = null;
  const learnButtons = [$('learn-btn'), $('learn-source-btn'), $('learn-source-osc-btn')];
  learnButtons.forEach(btn => btn?.classList.remove('learn-active'));
  hideLearnNotification();
}

function showLearnNotification(message) {
  // Create or update notification element
  let notification = $('learn-notification');
  if (!notification) {
    notification = document.createElement('div');
    notification.id = 'learn-notification';
    notification.className = 'learn-notification';
    document.body.appendChild(notification);
  }
  notification.textContent = message;
  notification.classList.add('visible');
}

function hideLearnNotification() {
  const notification = $('learn-notification');
  if (notification) {
    notification.classList.remove('visible');
  }
}

function handleLearnedSignal(signal) {
  if (!state.learnMode) return false;

  const modal = $('mapping-modal');
  if (!modal?.open) {
    // Open the modal and populate
    openMappingModal();
  }

  // Determine protocol from signal
  const protocol = detectProtocol(signal);

  if (state.learnTarget === 'source') {
    const protocolSelect = $('mapping-source-protocol');
    if (protocolSelect) {
      protocolSelect.value = protocol;
      updateProtocolFields('source', protocol);
    }

    // Fill in address based on protocol
    if (protocol === 'clasp' || protocol === 'osc') {
      // For CLASP/OSC, fill in the address
      if (protocol === 'clasp') {
        const claspAddressInput = document.querySelector('[name="sourceClaspAddress"]');
        if (claspAddressInput && signal.address) {
          claspAddressInput.value = signal.address;
        }
      } else {
        const oscAddressInput = document.querySelector('[name="sourceAddress"]');
        if (oscAddressInput && signal.address) {
          oscAddressInput.value = signal.address;
        }
      }
    } else if (protocol === 'midi') {
      // Parse MIDI info from signal
      const channelInput = document.querySelector('[name="sourceMidiChannel"]');
      const numberInput = document.querySelector('[name="sourceMidiNumber"]');
      const typeSelect = document.querySelector('[name="sourceMidiType"]');

      if (channelInput && signal.channel) channelInput.value = signal.channel;

      if (signal.note !== undefined) {
        if (typeSelect) typeSelect.value = 'note';
        if (numberInput) numberInput.value = signal.note;
      } else if (signal.cc !== undefined) {
        if (typeSelect) typeSelect.value = 'cc';
        if (numberInput) numberInput.value = signal.cc;
      }
    } else if (protocol === 'dmx' || protocol === 'artnet') {
      const universeInput = document.querySelector('[name="sourceDmxUniverse"]');
      const channelInput = document.querySelector('[name="sourceDmxChannel"]');
      if (universeInput && signal.universe !== undefined) universeInput.value = signal.universe;
      if (channelInput && signal.channel !== undefined) channelInput.value = signal.channel;
    } else if (protocol === 'mqtt') {
      // MQTT uses topic as address
      const claspAddressInput = document.querySelector('[name="sourceClaspAddress"]');
      if (claspAddressInput && signal.topic) {
        claspAddressInput.value = `/mqtt/${signal.topic}`;
      }
    }

    // Show success notification
    showLearnNotification(`Learned: ${signal.address || signal.topic || 'Signal'}`);
    setTimeout(hideLearnNotification, 2000);
  }

  resetLearnMode();
  return true;
}

function detectProtocol(signal) {
  // Check for CLASP namespace prefixes first
  if (signal.address?.startsWith('/mqtt/')) return 'mqtt';
  if (signal.address?.startsWith('/osc/')) return 'osc';
  if (signal.address?.startsWith('/')) return 'clasp'; // Default for OSC-like addresses
  if (signal.topic !== undefined) return 'mqtt';
  if (signal.channel !== undefined && (signal.note !== undefined || signal.cc !== undefined)) return 'midi';
  if (signal.universe !== undefined) return 'artnet';
  return 'clasp'; // default
}

// ============================================
// Event Listeners
// ============================================

function setupEventListeners() {
  // Event delegation for delete actions (CSP-compliant)
  document.addEventListener('click', (e) => {
    const target = e.target.closest('[data-action]');
    if (!target) return;

    const action = target.dataset.action;
    const id = target.dataset.id;

    switch (action) {
      case 'delete-server':
        deleteServer(id);
        break;
      case 'delete-bridge':
        deleteBridge(id);
        break;
      case 'delete-mapping':
        deleteMapping(id);
        break;
    }
  });

  // Scan button
  $('scan-btn')?.addEventListener('click', handleScan);

  // Add server button
  $('add-server-btn')?.addEventListener('click', () => {
    state.editingServer = null;
    const modalTitle = document.querySelector('#server-modal .modal-title');
    if (modalTitle) modalTitle.textContent = 'ADD SERVER';
    $('server-form')?.reset();
    $('server-modal')?.showModal();
  });

  // Server form
  $('server-form')?.addEventListener('submit', handleAddServer);

  // Server list actions (edit/delete)
  $('server-list')?.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action]');
    if (!btn) return;
    const action = btn.dataset.action;
    const id = btn.dataset.id;
    if (action === 'edit-server') editServer(id);
    if (action === 'delete-server') deleteServer(id);
  });

  // Add bridge button
  $('add-bridge-btn')?.addEventListener('click', () => {
    $('bridge-modal')?.showModal();
  });

  // Bridge form
  $('bridge-form')?.addEventListener('submit', handleCreateBridge);

  // Add mapping button
  $('add-mapping-btn')?.addEventListener('click', () => {
    state.editingMapping = null;
    openMappingModal();
  });

  // Mapping form
  $('mapping-form')?.addEventListener('submit', handleCreateMapping);

  // Monitor controls
  $('pause-btn')?.addEventListener('click', togglePause);
  $('clear-btn')?.addEventListener('click', clearSignals);
  $('monitor-filter')?.addEventListener('input', (e) => {
    state.monitorFilter = e.target.value.toLowerCase();
    renderSignalMonitor();
  });

  // IPC events
  if (window.clasp) {
    window.clasp.onDeviceFound?.((device) => {
      upsertDevice(device);
      renderDevices();
      updateStatus();
    });

    window.clasp.onDeviceUpdated?.((device) => {
      upsertDevice(device);
      renderDevices();
      updateStatus();
    });

    window.clasp.onDeviceLost?.((deviceId) => {
      state.devices = state.devices.filter(d => d.id !== deviceId);
      renderDevices();
      updateStatus();
    });

    window.clasp.onSignal?.((signal) => {
      // Check learn mode first
      if (handleLearnedSignal(signal)) return;

      // Otherwise add to monitor
      if (!state.paused) {
        addSignal(signal);
        applyMappings(signal);
      }
    });

    window.clasp.onScanStarted?.(() => {
      state.scanning = true;
      updateScanButton();
    });

    window.clasp.onScanComplete?.(() => {
      state.scanning = false;
      updateScanButton();
      loadDevices().then(renderDevices);
    });

    // Server status updates
    window.clasp.onServerStatus?.((status) => {
      const server = state.servers.find(s => s.id === status.id);
      if (server) {
        server.status = status.status;
        if (status.error) {
          server.error = status.error;
          showNotification(`Server error: ${status.error}`, 'error');
        }
        if (status.status === 'running') {
          showNotification(`Server started successfully`, 'success');
        }
        renderServers();
        updateStatus();
      }
    });

    // Server log updates
    window.clasp.onServerLog?.((data) => {
      if (!state.serverLogs.has(data.serverId)) {
        state.serverLogs.set(data.serverId, []);
      }
      const logs = state.serverLogs.get(data.serverId);
      logs.push(data.log);
      if (logs.length > 500) {
        logs.shift();
      }
    });

    // Bridge events
    window.clasp.onBridgeEvent?.((data) => {
      const bridge = state.bridges.find(b => b.id === data.bridgeId);
      if (bridge) {
        if (data.event === 'connected') {
          bridge.active = true;
          showNotification(`Bridge connected`, 'success');
        } else if (data.event === 'disconnected') {
          bridge.active = false;
          if (data.data) {
            showNotification(`Bridge disconnected: ${data.data}`, 'warning');
          }
        } else if (data.event === 'error') {
          showNotification(`Bridge error: ${data.data}`, 'error');
        }
        renderBridges();
      }
    });
  }
}

function upsertDevice(device) {
  const idx = state.devices.findIndex(d => d.id === device.id);
  if (idx >= 0) {
    state.devices[idx] = device;
  } else {
    state.devices.push(device);
  }
}

// ============================================
// Event Handlers
// ============================================

async function handleScan() {
  if (state.scanning) return;

  state.scanning = true;
  updateScanButton();

  try {
    if (window.clasp) {
      await window.clasp.scanNetwork();
    } else {
      await new Promise(r => setTimeout(r, 1500));
    }
  } finally {
    state.scanning = false;
    updateScanButton();
    await loadDevices();
    renderDevices();
  }
}

function updateScanButton() {
  const btn = $('scan-btn');
  if (!btn) return;

  if (state.scanning) {
    btn.innerHTML = `<svg class="icon spinning" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 11-6.219-8.56"/></svg> SCANNING`;
    btn.disabled = true;
  } else {
    btn.innerHTML = `${icons.scan} SCAN`;
    btn.disabled = false;
  }
}

async function handleAddServer(e) {
  e.preventDefault();
  const form = e.target;
  const data = new FormData(form);
  const serverType = data.get('serverType') || 'clasp';
  const isEditing = state.editingServer !== null;

  let serverConfig = {
    id: isEditing ? state.editingServer.id : Date.now().toString(),
    type: serverType,
    protocol: serverType,
    status: 'starting',
  };

  // Build config based on server type
  switch (serverType) {
    case 'clasp':
      serverConfig.address = data.get('claspAddress') || 'localhost:7330';
      serverConfig.token = data.get('claspToken') || '';
      serverConfig.serverName = data.get('claspName') || 'CLASP Bridge Server';
      serverConfig.announce = data.get('claspAnnounce') === 'on';
      serverConfig.name = data.get('claspName') || `CLASP Server @ ${serverConfig.address}`;
      break;

    case 'osc':
      serverConfig.bind = data.get('oscBind') || '0.0.0.0';
      serverConfig.port = parseInt(data.get('oscPort')) || 9000;
      serverConfig.address = `${serverConfig.bind}:${serverConfig.port}`;
      serverConfig.name = `OSC Server @ ${serverConfig.address}`;
      break;

    case 'mqtt':
      serverConfig.host = data.get('mqttHost') || 'localhost';
      serverConfig.port = parseInt(data.get('mqttPort')) || 1883;
      serverConfig.topics = (data.get('mqttTopics') || '#').split(',').map(t => t.trim());
      serverConfig.address = `${serverConfig.host}:${serverConfig.port}`;
      serverConfig.name = `MQTT Broker @ ${serverConfig.address}`;
      break;

    case 'websocket':
      serverConfig.mode = data.get('wsMode') || 'server';
      serverConfig.address = data.get('wsAddress') || '0.0.0.0:8080';
      serverConfig.name = `WebSocket ${serverConfig.mode === 'server' ? 'Server' : 'Client'} @ ${serverConfig.address}`;
      break;

    case 'http':
      serverConfig.bind = data.get('httpBind') || '0.0.0.0:3000';
      serverConfig.basePath = data.get('httpBasePath') || '/api';
      serverConfig.cors = data.get('httpCors') === 'on';
      serverConfig.address = serverConfig.bind;
      serverConfig.name = `HTTP REST API @ ${serverConfig.address}`;
      break;

    case 'artnet':
      serverConfig.bind = data.get('artnetBind') || '0.0.0.0:6454';
      serverConfig.subnet = parseInt(data.get('artnetSubnet')) || 0;
      serverConfig.universe = parseInt(data.get('artnetUniverse')) || 0;
      serverConfig.address = serverConfig.bind;
      serverConfig.name = `Art-Net @ ${serverConfig.address} (${serverConfig.subnet}:${serverConfig.universe})`;
      break;

    case 'dmx':
      serverConfig.serialPort = data.get('dmxPort') || '/dev/ttyUSB0';
      serverConfig.universe = parseInt(data.get('dmxUniverse')) || 0;
      serverConfig.address = serverConfig.serialPort;
      serverConfig.name = `DMX @ ${serverConfig.serialPort} (U${serverConfig.universe})`;
      break;

    case 'midi':
      serverConfig.inputPort = data.get('midiInput') || '';
      serverConfig.outputPort = data.get('midiOutput') || '';
      serverConfig.address = serverConfig.inputPort || serverConfig.outputPort || 'MIDI Device';
      serverConfig.name = `MIDI Bridge (${serverConfig.inputPort || 'no input'} / ${serverConfig.outputPort || 'no output'})`;
      break;

    case 'socketio':
      serverConfig.mode = data.get('socketioMode') || 'server';
      serverConfig.address = data.get('socketioAddress') || '0.0.0.0:3001';
      serverConfig.namespace = data.get('socketioNamespace') || '/';
      serverConfig.name = `Socket.IO ${serverConfig.mode === 'server' ? 'Server' : 'Client'} @ ${serverConfig.address}${serverConfig.namespace}`;
      break;

    default:
      console.error('Unknown server type:', serverType);
      return;
  }

  try {
    if (window.clasp) {
      // Call backend to start the server
      const result = await window.clasp.startServer(serverConfig);
      serverConfig.id = result?.id || serverConfig.id;
      serverConfig.status = 'connected';
    } else {
      // Mock mode
      serverConfig.status = 'connected';
    }

    // Update existing or add new
    if (isEditing) {
      const idx = state.servers.findIndex(s => s.id === serverConfig.id);
      if (idx !== -1) {
        state.servers[idx] = serverConfig;
      } else {
        state.servers.push(serverConfig);
      }
    } else {
      state.servers.push(serverConfig);
    }

    state.editingServer = null;
    saveServersToStorage();
    renderServers();
    updateStatus();
    $('server-modal')?.close();
    form.reset();
    updateServerTypeFields('clasp'); // Reset to default fields
  } catch (err) {
    console.error('Failed to start server:', err);
    serverConfig.status = 'error';
    serverConfig.error = err.message;

    if (isEditing) {
      const idx = state.servers.findIndex(s => s.id === serverConfig.id);
      if (idx !== -1) {
        state.servers[idx] = serverConfig;
      } else {
        state.servers.push(serverConfig);
      }
    } else {
      state.servers.push(serverConfig);
    }

    state.editingServer = null;
    saveServersToStorage();
    renderServers();
  }
}

async function deleteServer(id) {
  try {
    if (window.clasp) {
      await window.clasp.stopServer(id);
    }
    state.servers = state.servers.filter(s => s.id !== id);
    saveServersToStorage();
    renderServers();
    updateStatus();
  } catch (err) {
    console.error('Failed to delete server:', err);
  }
}

function editServer(id) {
  const server = state.servers.find(s => s.id === id);
  if (!server) return;

  state.editingServer = server;

  // Update modal title
  const modalTitle = document.querySelector('#server-modal .modal-title');
  if (modalTitle) modalTitle.textContent = 'EDIT SERVER';

  // Set server type
  const typeSelect = $('server-type');
  if (typeSelect) {
    typeSelect.value = server.protocol || server.type || 'clasp';
    typeSelect.dispatchEvent(new Event('change')); // Trigger field switching
  }

  // Populate fields based on server type
  const form = $('server-form');
  if (!form) return;

  switch (server.protocol || server.type) {
    case 'clasp':
      form.elements.claspAddress.value = server.address || 'localhost:7330';
      if (form.elements.claspToken) form.elements.claspToken.value = server.token || '';
      break;
    case 'osc':
      form.elements.oscBind.value = server.bind || '0.0.0.0';
      form.elements.oscPort.value = server.port || 9000;
      break;
    case 'mqtt':
      form.elements.mqttHost.value = server.host || 'localhost';
      form.elements.mqttPort.value = server.port || 1883;
      form.elements.mqttTopics.value = (server.topics || ['#']).join(', ');
      break;
    case 'websocket':
      form.elements.wsMode.value = server.mode || 'server';
      form.elements.wsAddress.value = server.address || '0.0.0.0:8080';
      break;
    case 'http':
      form.elements.httpBind.value = server.bind || '0.0.0.0:3000';
      form.elements.httpBasePath.value = server.basePath || '/api';
      form.elements.httpCors.checked = server.cors !== false;
      break;
    case 'artnet':
      form.elements.artnetBind.value = server.bind || '0.0.0.0:6454';
      form.elements.artnetSubnet.value = server.subnet || 0;
      form.elements.artnetUniverse.value = server.universe || 0;
      break;
    case 'dmx':
      form.elements.dmxPort.value = server.serialPort || '/dev/ttyUSB0';
      form.elements.dmxUniverse.value = server.universe || 0;
      break;
  }

  $('server-modal')?.showModal();
}

async function handleCreateBridge(e) {
  e.preventDefault();
  const form = e.target;
  const data = new FormData(form);

  const config = {
    source: data.get('source'),
    sourceAddr: data.get('sourceAddr') || defaultAddresses[data.get('source')],
    target: data.get('target'),
    targetAddr: data.get('targetAddr') || defaultAddresses[data.get('target')],
  };

  try {
    let bridge;
    if (window.clasp) {
      bridge = await window.clasp.createBridge(config);
    } else {
      bridge = { id: Date.now().toString(), ...config, active: true };
    }
    state.bridges.push(bridge);
    saveBridgesToStorage();
    renderBridges();
    $('bridge-modal')?.close();
    form.reset();
  } catch (err) {
    console.error('Failed to create bridge:', err);
  }
}

function openMappingModal() {
  const modal = $('mapping-modal');
  if (!modal) return;

  // Reset form
  $('mapping-form')?.reset();

  // Reset field visibility to defaults (CLASP is now first/default)
  updateProtocolFields('source', 'clasp');
  updateProtocolFields('target', 'clasp');

  // Reset transform params
  updateTransformParams('direct');

  // Reset value type visibility
  $('source-json-path-group')?.classList.add('hidden');
  $('target-json-template-group')?.classList.add('hidden');

  // Update preview
  updateTransformPreview();

  modal.showModal();
}

function handleCreateMapping(e) {
  e.preventDefault();
  const form = e.target;
  const data = new FormData(form);

  const sourceProtocol = data.get('sourceProtocol');
  const targetProtocol = data.get('targetProtocol');

  const mapping = {
    id: state.editingMapping || Date.now().toString(),
    enabled: true,
    source: {
      protocol: sourceProtocol,
      // CLASP or OSC address
      address: sourceProtocol === 'clasp' ? data.get('sourceClaspAddress') : data.get('sourceAddress') || null,
      // MIDI fields
      midiType: data.get('sourceMidiType') || null,
      midiChannel: parseInt(data.get('sourceMidiChannel')) || null,
      midiNumber: data.get('sourceMidiNumber') ? parseInt(data.get('sourceMidiNumber')) : null,
      // DMX fields
      dmxUniverse: parseInt(data.get('sourceDmxUniverse')) || null,
      dmxChannel: parseInt(data.get('sourceDmxChannel')) || null,
      // Value type
      valueType: data.get('sourceValueType') || 'auto',
      jsonPath: data.get('sourceJsonPath') || null,
    },
    target: {
      protocol: targetProtocol,
      // CLASP or OSC address
      address: targetProtocol === 'clasp' ? data.get('targetClaspAddress') : data.get('targetAddress') || null,
      // MIDI fields
      midiType: data.get('targetMidiType') || null,
      midiChannel: parseInt(data.get('targetMidiChannel')) || null,
      midiNumber: parseInt(data.get('targetMidiNumber')) || null,
      // DMX fields
      dmxUniverse: parseInt(data.get('targetDmxUniverse')) || null,
      dmxChannel: parseInt(data.get('targetDmxChannel')) || null,
      // Value type
      valueType: data.get('targetValueType') || 'auto',
      jsonTemplate: data.get('targetJsonTemplate') || null,
    },
    transform: {
      type: data.get('transform'),
      // Scale params
      scaleInMin: parseFloat(data.get('scaleInMin')) || 0,
      scaleInMax: parseFloat(data.get('scaleInMax')) || 1,
      scaleOutMin: parseFloat(data.get('scaleOutMin')) || 0,
      scaleOutMax: parseFloat(data.get('scaleOutMax')) || 127,
      // Clamp params
      clampMin: parseFloat(data.get('clampMin')) || 0,
      clampMax: parseFloat(data.get('clampMax')) || 1,
      // Threshold
      threshold: parseFloat(data.get('threshold')) || 0.5,
      // Expression
      expression: data.get('expression') || null,
      // JavaScript
      javascriptCode: data.get('javascriptCode') || null,
    },
  };

  // Add or update
  if (state.editingMapping) {
    const idx = state.mappings.findIndex(m => m.id === state.editingMapping);
    if (idx >= 0) state.mappings[idx] = mapping;
  } else {
    state.mappings.push(mapping);
  }

  saveMappingsToStorage();
  renderMappings();
  updateMappingCount();
  $('mapping-modal')?.close();
  state.editingMapping = null;
}

function deleteMapping(id) {
  state.mappings = state.mappings.filter(m => m.id !== id);
  saveMappingsToStorage();
  renderMappings();
  updateMappingCount();
}

async function deleteBridge(id) {
  try {
    if (window.clasp) {
      await window.clasp.deleteBridge(id);
    }
    state.bridges = state.bridges.filter(b => b.id !== id);
    saveBridgesToStorage();
    renderBridges();
  } catch (err) {
    console.error('Failed to delete bridge:', err);
  }
}

function togglePause() {
  state.paused = !state.paused;
  const btn = $('pause-btn');
  if (btn) {
    btn.innerHTML = state.paused ? icons.play : icons.pause;
    btn.title = state.paused ? 'Resume' : 'Pause';
  }
}

function clearSignals() {
  state.signals = [];
  renderSignalMonitor();
}

// ============================================
// Signal Processing & Mapping
// ============================================

function addSignal(signal) {
  signalCount++;

  state.signals.unshift({
    ...signal,
    timestamp: Date.now(),
  });

  if (state.signals.length > 200) {
    state.signals = state.signals.slice(0, 200);
  }

  renderSignalMonitor();
}

function applyMappings(signal) {
  for (const mapping of state.mappings) {
    if (!mapping.enabled) continue;
    if (!matchesSource(signal, mapping.source)) continue;

    // Get value from signal
    let value = extractValue(signal, mapping.source);

    // Apply transform
    value = applyTransform(value, mapping.transform);

    // Build target address
    const targetAddress = buildTargetAddress(mapping.target, signal);

    // Send to target via appropriate bridge
    sendToTarget(mapping.target, targetAddress, value);
  }
}

function buildTargetAddress(target, sourceSignal) {
  switch (target.protocol) {
    case 'clasp':
    case 'osc':
      return target.address || sourceSignal.address || '/*';
    case 'midi':
      // MIDI doesn't use addresses
      return null;
    case 'dmx':
    case 'artnet':
      return null;
    default:
      return target.address;
  }
}

async function sendToTarget(target, address, value) {
  if (!window.clasp?.sendSignal) {
    console.log(`Would send: ${address} = ${value} (${target.protocol})`);
    return;
  }

  // Find an appropriate bridge for this target protocol
  const bridge = state.bridges.find(b =>
    b.target === target.protocol || b.source === target.protocol
  );

  if (!bridge) {
    console.warn(`No bridge found for ${target.protocol}`);
    return;
  }

  try {
    await window.clasp.sendSignal(bridge.id, address, value);
  } catch (err) {
    console.error('Failed to send signal:', err);
  }
}

function matchesSource(signal, source) {
  switch (source.protocol) {
    case 'clasp':
    case 'osc':
      if (!signal.address) return false;
      if (source.address) {
        // Support CLASP wildcards: * for single segment, ** for multiple
        let pattern = source.address
          .replace(/\*\*/g, '§§')  // Temp placeholder
          .replace(/\*/g, '[^/]+')  // Single wildcard
          .replace(/§§/g, '.*');    // Multi wildcard
        return new RegExp(`^${pattern}$`).test(signal.address);
      }
      return true;

    case 'midi':
      if (source.midiChannel && signal.channel !== source.midiChannel) return false;
      if (source.midiNumber !== null && signal.note !== source.midiNumber && signal.cc !== source.midiNumber) return false;
      return true;

    case 'dmx':
    case 'artnet':
      if (source.dmxUniverse !== null && signal.universe !== source.dmxUniverse) return false;
      if (source.dmxChannel !== null && signal.channel !== source.dmxChannel) return false;
      return true;

    default:
      return false;
  }
}

function extractValue(signal, source) {
  let value;

  // Get raw value
  if (typeof signal.value === 'number') {
    value = signal.value;
  } else if (signal.velocity !== undefined) {
    value = signal.velocity / 127;
  } else if (signal.value !== undefined) {
    value = signal.value;
  } else {
    value = 0;
  }

  // Apply JSON path extraction if specified
  if (source.jsonPath && typeof value === 'object') {
    try {
      value = extractJsonPath(value, source.jsonPath);
    } catch (e) {
      console.warn('JSON path extraction failed:', e);
    }
  }

  return value;
}

function extractJsonPath(obj, path) {
  // Simple JSON path implementation (supports $.foo.bar[0] style)
  const parts = path.replace(/^\$\.?/, '').split(/\.|\[|\]/).filter(p => p);
  let result = obj;
  for (const part of parts) {
    if (result === null || result === undefined) return undefined;
    result = result[part];
  }
  return result;
}

function applyTransform(value, transform) {
  switch (transform.type) {
    case 'direct':
      return value;

    case 'scale': {
      // Map from input range to output range
      const normalized = (value - transform.scaleInMin) / (transform.scaleInMax - transform.scaleInMin);
      return transform.scaleOutMin + normalized * (transform.scaleOutMax - transform.scaleOutMin);
    }

    case 'invert':
      return 1 - value;

    case 'clamp':
      return Math.min(transform.clampMax, Math.max(transform.clampMin, value));

    case 'round':
      return Math.round(value);

    case 'toggle':
      return value > 0.5 ? 1 : 0;

    case 'gate':
      return value > 0 ? 1 : 0;

    case 'threshold':
      return value >= transform.threshold ? 1 : 0;

    case 'expression':
      try {
        return evaluateExpression(transform.expression, value);
      } catch (e) {
        console.error('Expression evaluation failed:', e);
        return value;
      }

    case 'javascript':
      try {
        const fn = new Function('input', `
          ${transform.javascriptCode}
          return transform(input);
        `);
        return fn(value);
      } catch (e) {
        console.error('JavaScript transform failed:', e);
        return value;
      }

    default:
      return value;
  }
}

// ============================================
// Rendering
// ============================================

function renderServers() {
  const list = $('server-list');
  if (!list) return;

  if (state.servers.length === 0) {
    list.innerHTML = `
      <div class="empty-state-small">
        <span class="empty-state-text">No servers running</span>
      </div>
    `;
  } else {
    list.innerHTML = state.servers.map(server => `
      <div class="device-item" data-id="${server.id}">
        <span class="status-dot ${server.status || 'available'}"></span>
        <span class="device-protocol-badge ${server.protocol || 'clasp'}">${protocolNames[server.protocol] || server.protocol || 'CLASP'}</span>
        <span class="device-name">${server.name}</span>
        <div class="device-actions">
          <button class="btn-device-edit" data-action="edit-server" data-id="${server.id}" title="Edit server">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button class="btn-device-delete" data-action="delete-server" data-id="${server.id}" title="Stop server">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </div>
    `).join('');
  }

  // Update badge
  const badge = $('server-badge');
  if (badge) badge.textContent = state.servers.length;
}

function renderDevices() {
  const list = $('device-list');
  if (!list) return;

  if (state.devices.length === 0) {
    list.innerHTML = `
      <div class="empty-state-small">
        <span class="empty-state-text">No devices found</span>
      </div>
    `;
    return;
  }

  list.innerHTML = state.devices.map(device => `
    <div class="device-item" data-id="${device.id}">
      <span class="status-dot ${device.status || 'available'}"></span>
      <span class="device-protocol-badge ${device.protocol || 'clasp'}">${protocolNames[device.protocol] || device.protocol || 'CLASP'}</span>
      <span class="device-name">${device.name}</span>
    </div>
  `).join('');

  // Update badge
  const badge = $('device-badge');
  if (badge) badge.textContent = state.devices.length;
}

function renderBridges() {
  const list = $('bridge-list');
  if (!list) return;

  if (state.bridges.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-icon">${icons.bridge}</div>
        <div class="empty-state-text">No bridges configured</div>
        <div class="empty-state-hint">Create a bridge to connect protocols</div>
      </div>
    `;
    return;
  }

  list.innerHTML = state.bridges.map(bridge => `
    <div class="bridge-card" data-id="${bridge.id}">
      <div class="bridge-endpoint">
        <span class="bridge-endpoint-label">${protocolNames[bridge.source] || bridge.source}</span>
        <span class="bridge-endpoint-value">${bridge.sourceAddr || '--'}</span>
      </div>
      ${icons.arrow}
      <div class="bridge-endpoint">
        <span class="bridge-endpoint-label">${protocolNames[bridge.target] || bridge.target}</span>
        <span class="bridge-endpoint-value">${bridge.targetAddr || '--'}</span>
      </div>
      <div class="bridge-actions">
        <button class="btn btn-sm btn-delete" data-action="delete-bridge" data-id="${bridge.id}" title="Delete">
          ${icons.delete}
        </button>
      </div>
    </div>
  `).join('');
}

function renderMappings() {
  const list = $('mapping-list');
  if (!list) return;

  if (state.mappings.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-icon">${icons.mapping}</div>
        <div class="empty-state-text">No mappings configured</div>
        <div class="empty-state-hint">Create mappings to route signals between protocols</div>
      </div>
    `;
    return;
  }

  list.innerHTML = state.mappings.map(mapping => `
    <div class="mapping-item" data-id="${mapping.id}">
      <div class="mapping-source">
        <span class="mapping-protocol">${protocolNames[mapping.source.protocol]}</span>
        <span class="mapping-address">${formatMappingEndpoint(mapping.source)}</span>
      </div>
      <span class="mapping-transform-badge">${formatTransform(mapping.transform)}</span>
      <div class="mapping-target">
        <span class="mapping-protocol">${protocolNames[mapping.target.protocol]}</span>
        <span class="mapping-address">${formatMappingEndpoint(mapping.target)}</span>
      </div>
      <div class="bridge-actions">
        <button class="btn btn-sm btn-delete" data-action="delete-mapping" data-id="${mapping.id}" title="Delete">
          ${icons.delete}
        </button>
      </div>
    </div>
  `).join('');
}

function formatMappingEndpoint(endpoint) {
  switch (endpoint.protocol) {
    case 'clasp':
      return endpoint.address || '/clasp/*';
    case 'osc':
      return endpoint.address || '/*';
    case 'midi':
      const type = endpoint.midiType || 'note';
      const ch = endpoint.midiChannel || '*';
      const num = endpoint.midiNumber !== null ? endpoint.midiNumber : '*';
      return `Ch${ch} ${type.toUpperCase()} ${num}`;
    case 'dmx':
    case 'artnet':
      const uni = endpoint.dmxUniverse !== null ? endpoint.dmxUniverse : '*';
      const chan = endpoint.dmxChannel !== null ? endpoint.dmxChannel : '*';
      return `U${uni} Ch${chan}`;
    default:
      return '--';
  }
}

function formatTransform(transform) {
  switch (transform.type) {
    case 'direct': return '→';
    case 'scale': return `${transform.scaleInMin}-${transform.scaleInMax} → ${transform.scaleOutMin}-${transform.scaleOutMax}`;
    case 'invert': return '↔ INV';
    case 'toggle': return '⊡ TOG';
    case 'threshold': return `≥${transform.threshold}`;
    case 'clamp': return `[${transform.clampMin}..${transform.clampMax}]`;
    case 'round': return '⌊x⌋';
    case 'gate': return '⊐ GATE';
    case 'trigger': return '⌁ TRIG';
    case 'expression': return `f(x)`;
    case 'javascript': return 'JS( )';
    default: return '→';
  }
}

function renderSignalMonitor() {
  const monitor = $('signal-monitor');
  if (!monitor) return;

  // Filter signals
  let signals = state.signals;
  if (state.monitorFilter) {
    signals = signals.filter(s =>
      (s.address && s.address.toLowerCase().includes(state.monitorFilter)) ||
      (s.bridgeId && s.bridgeId.toLowerCase().includes(state.monitorFilter))
    );
  }

  if (signals.length === 0) {
    monitor.innerHTML = `
      <div class="signal-empty">
        <span>${state.monitorFilter ? 'No matching signals' : 'Waiting for signals...'}</span>
      </div>
    `;
    return;
  }

  monitor.innerHTML = signals.slice(0, 100).map(s => {
    const val = typeof s.value === 'number' ? s.value : 0;
    const percent = Math.min(100, Math.max(0, Math.abs(val) * 100));
    const displayVal = formatSignalValue(s.value);

    return `
      <div class="signal-item">
        <span class="signal-address">${s.address || s.bridgeId || '--'}</span>
        <span class="signal-value">${displayVal}</span>
        <div class="signal-bar">
          <div class="signal-bar-fill" style="width: ${percent}%"></div>
        </div>
      </div>
    `;
  }).join('');
}

function formatSignalValue(value) {
  if (typeof value === 'number') {
    return value % 1 === 0 ? value.toString() : value.toFixed(3);
  }
  if (typeof value === 'boolean') {
    return value ? 'ON' : 'OFF';
  }
  if (Array.isArray(value)) {
    return `[${value.length}]`;
  }
  if (typeof value === 'object') {
    return '{...}';
  }
  return String(value);
}

function updateStatus() {
  const connected = state.devices.filter(d => d.status === 'connected').length;

  const deviceCount = $('device-count');
  if (deviceCount) deviceCount.textContent = connected;

  const indicator = $('status-indicator');
  if (indicator) {
    indicator.className = connected > 0 ? 'status-indicator connected' : 'status-indicator';
  }
}

function updateMappingCount() {
  const count = $('mapping-count');
  if (count) count.textContent = state.mappings.length;
}

// Signal rate tracking
function updateSignalRate() {
  state.signalRate = signalCount;
  signalCount = 0;

  const rateEl = $('signal-rate');
  if (rateEl) rateEl.textContent = state.signalRate;

  const rateStat = $('rate-stat');
  if (rateStat) rateStat.textContent = `${state.signalRate}/s`;
}

// ============================================
// Notifications
// ============================================

function showNotification(message, type = 'info') {
  // Create notification container if not exists
  let container = $('notification-container');
  if (!container) {
    container = document.createElement('div');
    container.id = 'notification-container';
    container.className = 'notification-container';
    document.body.appendChild(container);
  }

  // Create notification element
  const notification = document.createElement('div');
  notification.className = `notification notification-${type}`;

  const icon = type === 'success' ? '✓' : type === 'error' ? '✕' : type === 'warning' ? '!' : 'ℹ';

  notification.innerHTML = `
    <span class="notification-icon">${icon}</span>
    <span class="notification-message">${escapeHtml(message)}</span>
    <button class="notification-close">×</button>
  `;

  // Add click handler for close button
  notification.querySelector('.notification-close').addEventListener('click', () => {
    notification.classList.add('fade-out');
    setTimeout(() => notification.remove(), 300);
  });

  container.appendChild(notification);

  // Auto-remove after 5 seconds
  setTimeout(() => {
    if (notification.parentNode) {
      notification.classList.add('fade-out');
      setTimeout(() => notification.remove(), 300);
    }
  }, 5000);
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ============================================
// Global Functions (for onclick handlers)
// ============================================

// No longer needed - using event delegation instead
// window.deleteBridge = deleteBridge;
// window.deleteMapping = deleteMapping;
// window.deleteServer = deleteServer;

// ============================================
// Initialize
// ============================================

document.addEventListener('DOMContentLoaded', init);
