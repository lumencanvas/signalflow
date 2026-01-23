/**
 * CLASP Bridge - Main Application v2
 * Full-featured protocol mapping and bridging
 */

// Import presets and config helpers
import { presets, categories, getPreset } from './presets/index.js';
import { exportConfig, importConfig, downloadConfig, loadConfigFromFile, mergeConfig } from './lib/config-io.js';

// State
const state = {
  routers: [],      // CLASP routers (central message hubs)
  servers: [],      // Protocol connections (OSC, MIDI, MQTT, etc. - connected to routers)
  devices: [],      // Discovered devices
  bridges: [],
  mappings: [],
  signals: [],
  serverLogs: new Map(), // Server ID -> log entries
  systemLogs: [],   // Global system logs
  signalRate: 0,
  paused: false,
  scanning: false,
  bridgeServiceReady: false, // Whether the clasp-service binary is running and ready
  activeTab: 'bridges',
  learnMode: false,
  learnTarget: null, // 'source' or 'target'
  editingMapping: null,
  editingServer: null, // Server being edited
  editingRouter: null, // Router being edited
  monitorFilter: '',
  protocolFilter: 'all', // Protocol filter for monitor
  maxSignals: 200, // Max signals to keep in monitor (auto-clear)
  // Signal history for sparklines
  signalHistory: new Map(), // address -> { values: [], lastUpdate: timestamp }
  // Onboarding
  onboardingStep: 1,
  selectedUseCase: null,
  // Server stats (updated from backend)
  serverStats: new Map(), // id -> stats object
  // Continuous test mode
  continuousTestInterval: null,
  // Token management for CLASP server auth
  tokens: [],  // { id, name, token, scopes: ['read:/**', 'write:/**'] }
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
  edit: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>',
  arrow: '<svg class="bridge-arrow" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>',
  bridge: '<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M4 12h16M8 8l-4 4 4 4M16 8l4 4-4 4"/></svg>',
  mapping: '<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="6" cy="12" r="3"/><circle cx="18" cy="12" r="3"/><line x1="9" y1="12" x2="15" y2="12"/></svg>',
};

// Protocol display names
const protocolNames = {
  osc: 'OSC',
  midi: 'MIDI',
  artnet: 'Art-Net',
  sacn: 'sACN',
  dmx: 'DMX',
  clasp: 'CLASP',
  mqtt: 'MQTT',
  websocket: 'WS',
  socketio: 'SIO',
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

// Track intervals for cleanup
let rateCounterInterval = null;

// Initialize application
async function init() {
  // Load saved data from localStorage
  loadMappingsFromStorage();
  loadMaxSignalsSetting();

  // Wait for bridge service to be ready before restoring protocol connections
  await waitForBridgeService();

  // Restore saved routers, servers, and bridges (reconnect them)
  await restoreRoutersOnStartup();
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
  setupTokenManagement();
  setupHardwareDiscovery();
  setupTransformParams();
  setupLearnMode();
  setupPresetPicker();
  setupOnboarding();
  setupConfigButtons();
  setupLogViewer();
  setupFlowDiagram();
  setupTestPanel();
  setupServerStatsUpdates();

  // Initial render
  renderRouters();
  renderServers();
  renderDevices();
  renderBridges();
  renderMappings();
  renderSignalMonitor();
  renderFlowDiagram();
  renderLogs();
  renderServerHealth();
  updateStatus();
  updateMappingCount();

  // Start rate counter (clear any previous interval first)
  if (rateCounterInterval) clearInterval(rateCounterInterval);
  rateCounterInterval = setInterval(updateSignalRate, 1000);

  // Check for first run
  checkFirstRun();
}

// ============================================
// Bridge Service Readiness
// ============================================

async function waitForBridgeService(timeoutMs = 10000) {
  if (!window.clasp) {
    console.log('No clasp API available, skipping bridge service wait');
    return false;
  }

  const startTime = Date.now();
  
  // Check initial status
  try {
    const status = await window.clasp.getBridgeStatus();
    if (status.ready) {
      state.bridgeServiceReady = true;
      console.log('Bridge service already ready');
      return true;
    }
  } catch (e) {
    console.warn('Failed to check initial bridge status:', e);
  }

  // Poll for readiness
  return new Promise((resolve) => {
    const checkInterval = setInterval(async () => {
      try {
        const status = await window.clasp.getBridgeStatus();
        if (status.ready) {
          clearInterval(checkInterval);
          state.bridgeServiceReady = true;
          console.log('Bridge service became ready');
          resolve(true);
          return;
        }
      } catch (e) {
        // Ignore errors during polling
      }

      if (Date.now() - startTime > timeoutMs) {
        clearInterval(checkInterval);
        console.warn('Bridge service did not become ready within timeout');
        state.bridgeServiceReady = false;
        resolve(false);
      }
    }, 200);
  });
}

// ============================================
// Data Loading
// ============================================

async function loadDevices() {
  try {
    if (window.clasp) {
      const backendDevices = await window.clasp.getDevices();
      // Merge with existing devices, avoiding duplicates
      for (const device of backendDevices) {
        const existing = state.devices.find(d => d.id === device.id || (d.host === device.host && d.port === device.port));
        if (!existing) {
          state.devices.push(device);
        } else {
          // Update existing device with latest info
          Object.assign(existing, device);
        }
      }
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

function loadRoutersFromStorage() {
  try {
    const saved = localStorage.getItem('clasp-routers');
    if (saved) {
      const routers = JSON.parse(saved);
      return routers.map(r => ({ ...r, status: 'disconnected' }));
    }
  } catch (e) {
    console.error('Failed to load routers from storage:', e);
  }
  return [];
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

function saveRoutersToStorage() {
  try {
    const routersToSave = state.routers.map(r => ({
      id: r.id,
      type: r.type,
      protocol: r.protocol,
      name: r.name,
      address: r.address,
      announce: r.announce,
      authEnabled: r.authEnabled,
      token: r.token,
      // Remote router fields
      isRemote: r.isRemote || false,
      remoteAddress: r.remoteAddress,
      discoveredFrom: r.discoveredFrom,
    }));
    localStorage.setItem('clasp-routers', JSON.stringify(routersToSave));
  } catch (e) {
    console.error('Failed to save routers:', e);
  }
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
      routerId: s.routerId, // Which router this connects to
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

async function restoreRoutersOnStartup() {
  const savedRouters = loadRoutersFromStorage();
  for (const routerConfig of savedRouters) {
    // Remote routers don't need to be started - just add them to state
    if (routerConfig.isRemote) {
      routerConfig.status = 'available';
      state.routers.push(routerConfig);
      continue;
    }
    
    // Local routers need to be started
    try {
      if (window.clasp) {
        const result = await window.clasp.startServer(routerConfig);
        routerConfig.id = result?.id || routerConfig.id;
        routerConfig.status = 'connected';
      } else {
        routerConfig.status = 'connected';
      }
      state.routers.push(routerConfig);
    } catch (err) {
      console.warn(`Failed to restore router ${routerConfig.name}:`, err);
      routerConfig.status = 'error';
      routerConfig.error = err.message;
      state.routers.push(routerConfig);
    }
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
  // Clear existing bridges first to prevent duplicates
  state.bridges = [];
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

  // MQTT auth toggle
  $('mqtt-auth-enabled')?.addEventListener('change', (e) => {
    const authFields = $('mqtt-auth-fields');
    if (authFields) {
      authFields.classList.toggle('hidden', !e.target.checked);
    }
  });

  // sACN multicast toggle (show unicast fields when unchecked)
  document.querySelector('[name="sacnMulticast"]')?.addEventListener('change', (e) => {
    const unicastFields = $('sacn-unicast-fields');
    if (unicastFields) {
      unicastFields.classList.toggle('hidden', e.target.checked);
    }
  });
}

function updateServerTypeFields(serverType) {
  // Hide all server fields
  const allFields = ['clasp', 'osc', 'midi', 'mqtt', 'websocket', 'socketio', 'http', 'artnet', 'sacn', 'dmx'];
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
    clasp: 'CLASP router - central message hub that routes signals between clients',
    osc: 'OSC connection - receive OSC messages from controllers and translate to CLASP',
    midi: 'MIDI connection - connect to MIDI devices and translate to/from CLASP signals',
    mqtt: 'MQTT connection - connect to an MQTT broker with full auth and QoS support',
    websocket: 'WebSocket connection - accept JSON or MsgPack messages from web apps',
    socketio: 'Socket.IO connection - real-time bidirectional event-based communication',
    http: 'HTTP connection - expose signals as HTTP endpoints for webhooks and integrations',
    artnet: 'Art-Net connection - receive DMX512 data over Ethernet from lighting consoles',
    sacn: 'sACN/E1.31 connection - industry-standard streaming ACN for professional lighting',
    dmx: 'DMX connection - connect directly to DMX fixtures via USB adapter',
  };
  const hintEl = $('server-type-hint');
  if (hintEl) {
    hintEl.textContent = hints[serverType] || '';
  }

  // Populate hardware dropdowns when switching to relevant types
  if (serverType === 'midi') {
    refreshMidiPorts();
  } else if (serverType === 'dmx') {
    refreshSerialPorts();
  } else if (serverType === 'osc' || serverType === 'artnet' || serverType === 'http') {
    refreshNetworkInterfaces();
  }

  // Populate router dropdowns for protocol connections (not CLASP routers)
  if (serverType !== 'clasp') {
    populateRouterDropdowns();
  }

  // Update submit button text
  const submitBtn = $('server-form-submit');
  if (submitBtn) {
    if (serverType === 'clasp') {
      submitBtn.textContent = 'START ROUTER';
    } else {
      submitBtn.textContent = 'START CONNECTION';
    }
  }
}

// Populate router selection dropdowns in protocol connection modals
function populateRouterDropdowns() {
  const routerSelectors = [
    'osc-router-select',
    'midi-router-select',
    'mqtt-router-select',
    'websocket-router-select',
    'http-router-select',
    'artnet-router-select',
    'sacn-router-select',
    'dmx-router-select',
    'socketio-router-select',
  ];

  routerSelectors.forEach(selectorId => {
    const select = $(selectorId);
    if (!select) return;

    // Clear existing options except the first one
    while (select.options.length > 1) {
      select.remove(1);
    }

    // Get available routers: local running/connected routers + remote routers
    const availableRouters = state.routers.filter(r => 
      r.isRemote || r.status === 'connected' || r.status === 'running' || r.status === 'available'
    );
    
    if (availableRouters.length === 0) {
      // If no routers, show warning
      const option = document.createElement('option');
      option.value = '';
      option.textContent = 'No routers available - add a router first';
      option.disabled = true;
      select.appendChild(option);
    } else {
      availableRouters.forEach(router => {
        const option = document.createElement('option');
        option.value = router.id;
        const prefix = router.isRemote ? '↗ ' : ''; // Arrow to indicate remote
        const address = router.remoteAddress || router.address || '';
        option.textContent = `${prefix}${router.name}${address ? ` @ ${address}` : ''}`;
        select.appendChild(option);
      });
    }
  });
}

// ============================================
// Token Management (CLASP Server Auth)
// ============================================

function setupTokenManagement() {
  // Toggle token management visibility when auth checkbox changes
  const authCheckbox = $('clasp-auth-enabled');
  const tokenManagement = $('clasp-token-management');

  authCheckbox?.addEventListener('change', (e) => {
    if (tokenManagement) {
      tokenManagement.classList.toggle('hidden', !e.target.checked);
    }
  });

  // Create token button
  $('create-token-btn')?.addEventListener('click', () => {
    const dialog = $('create-token-dialog');
    if (dialog) {
      dialog.classList.remove('hidden');
      $('new-token-name')?.focus();
    }
  });

  // Cancel token creation
  $('cancel-token-btn')?.addEventListener('click', () => {
    $('create-token-dialog')?.classList.add('hidden');
    resetTokenDialog();
  });

  // Confirm token creation
  $('confirm-token-btn')?.addEventListener('click', () => {
    createNewToken();
  });

  // Load tokens from localStorage
  loadTokens();
  renderTokenList();
}

function generateCpskToken() {
  // Generate a CPSK token: cpsk_<32 base62 chars>
  const chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
  let random = '';
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  for (let i = 0; i < 32; i++) {
    random += chars[array[i] % chars.length];
  }
  return `cpsk_${random}`;
}

function createNewToken() {
  const name = $('new-token-name')?.value?.trim() || 'Unnamed Token';
  const pattern = $('new-token-pattern')?.value?.trim() || '/**';

  // Get selected permissions
  const readChecked = document.querySelector('[name="scope-read"]')?.checked;
  const writeChecked = document.querySelector('[name="scope-write"]')?.checked;
  const adminChecked = document.querySelector('[name="scope-admin"]')?.checked;

  // Build scopes
  const scopes = [];
  if (adminChecked) {
    scopes.push(`admin:${pattern}`);
  } else {
    if (readChecked) scopes.push(`read:${pattern}`);
    if (writeChecked) scopes.push(`write:${pattern}`);
  }

  if (scopes.length === 0) {
    alert('Please select at least one permission');
    return;
  }

  // Generate token
  const token = generateCpskToken();

  // Add to state
  const tokenEntry = {
    id: Date.now().toString(),
    name,
    token,
    scopes,
    created: new Date().toISOString(),
  };
  state.tokens.push(tokenEntry);
  saveTokens();
  renderTokenList();

  // Hide dialog and show token (only shown once!)
  $('create-token-dialog')?.classList.add('hidden');
  resetTokenDialog();

  // Show the token to the user (they need to copy it)
  showCreatedToken(tokenEntry);
}

function showCreatedToken(tokenEntry) {
  // Create a temporary success message with the token
  const tokenList = $('token-list');
  if (!tokenList) return;

  // Remove any existing success message
  tokenList.querySelector('.token-created')?.remove();

  const successEl = document.createElement('div');
  successEl.className = 'token-created';
  successEl.innerHTML = `
    <div class="token-created-header">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M20 6L9 17l-5-5"/>
      </svg>
      Token Created: ${escapeHtml(tokenEntry.name)}
    </div>
    <div class="token-created-value" onclick="navigator.clipboard.writeText(this.textContent).then(() => this.style.background='#d1fae5')">${tokenEntry.token}</div>
    <div class="token-created-warning">Copy this token now! It won't be shown again.</div>
  `;
  tokenList.insertBefore(successEl, tokenList.firstChild);

  // Auto-remove after 30 seconds
  setTimeout(() => successEl.remove(), 30000);
}

function deleteToken(id) {
  if (!confirm('Delete this token? Any clients using it will be disconnected.')) return;
  state.tokens = state.tokens.filter(t => t.id !== id);
  saveTokens();
  renderTokenList();
}

function copyToken(token) {
  navigator.clipboard.writeText(token).then(() => {
    // Brief visual feedback
    const btn = document.querySelector(`[data-copy-token="${token}"]`);
    if (btn) {
      const orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(() => btn.textContent = orig, 1500);
    }
  });
}

function renderTokenList() {
  const tokenList = $('token-list');
  if (!tokenList) return;

  // Keep any success messages
  const successMsg = tokenList.querySelector('.token-created');

  if (state.tokens.length === 0) {
    tokenList.innerHTML = '<div class="token-empty">No tokens yet. Create one to allow clients to connect.</div>';
    if (successMsg) tokenList.insertBefore(successMsg, tokenList.firstChild);
    return;
  }

  tokenList.innerHTML = state.tokens.map(t => `
    <div class="token-item" data-token-id="${t.id}">
      <div class="token-info">
        <div class="token-name">${escapeHtml(t.name)}</div>
        <div class="token-scopes">
          ${t.scopes.map(s => {
            const [action] = s.split(':');
            return `<span class="token-scope ${action}">${escapeHtml(s)}</span>`;
          }).join('')}
        </div>
        <div class="token-value">${t.token.substring(0, 20)}...</div>
      </div>
      <div class="token-actions">
        <button class="btn btn-secondary" data-copy-token="${t.token}" onclick="copyToken('${t.token}')">Copy</button>
        <button class="btn btn-secondary" onclick="deleteToken('${t.id}')">Delete</button>
      </div>
    </div>
  `).join('');

  if (successMsg) tokenList.insertBefore(successMsg, tokenList.firstChild);
}

function resetTokenDialog() {
  const nameInput = $('new-token-name');
  const patternInput = $('new-token-pattern');
  if (nameInput) nameInput.value = '';
  if (patternInput) patternInput.value = '/**';

  // Reset checkboxes
  document.querySelector('[name="scope-read"]').checked = true;
  document.querySelector('[name="scope-write"]').checked = true;
  document.querySelector('[name="scope-admin"]').checked = false;
}

function saveTokens() {
  try {
    localStorage.setItem('clasp-tokens', JSON.stringify(state.tokens));
  } catch (e) {
    console.error('Failed to save tokens:', e);
  }
}

function loadTokens() {
  try {
    const saved = localStorage.getItem('clasp-tokens');
    if (saved) {
      state.tokens = JSON.parse(saved);
    }
  } catch (e) {
    console.error('Failed to load tokens:', e);
    state.tokens = [];
  }
}

// Get tokens formatted for the router (token + scopes per line)
function getTokenFileContent() {
  return state.tokens.map(t => `${t.token} ${t.scopes.join(',')}`).join('\n');
}

// ============================================
// Hardware Discovery
// ============================================

async function refreshMidiPorts() {
  if (!window.clasp) return;

  try {
    const ports = await window.clasp.listMidiPorts();

    // Populate input select
    const inputSelect = $('midi-input-select');
    if (inputSelect) {
      const currentValue = inputSelect.value;
      inputSelect.innerHTML = '';
      for (const port of ports.inputs) {
        const option = document.createElement('option');
        option.value = port.id;
        option.textContent = port.name;
        inputSelect.appendChild(option);
      }
      // Restore previous value if still available
      if ([...inputSelect.options].some(o => o.value === currentValue)) {
        inputSelect.value = currentValue;
      }
    }

    // Populate output select
    const outputSelect = $('midi-output-select');
    if (outputSelect) {
      const currentValue = outputSelect.value;
      outputSelect.innerHTML = '<option value="">None (input only)</option>';
      for (const port of ports.outputs) {
        const option = document.createElement('option');
        option.value = port.id;
        option.textContent = port.name;
        outputSelect.appendChild(option);
      }
      if ([...outputSelect.options].some(o => o.value === currentValue)) {
        outputSelect.value = currentValue;
      }
    }
  } catch (e) {
    console.error('Failed to list MIDI ports:', e);
  }
}

async function refreshSerialPorts() {
  if (!window.clasp) return;

  try {
    const ports = await window.clasp.listSerialPorts();

    const select = $('dmx-port-select');
    if (select) {
      const currentValue = select.value;
      select.innerHTML = '<option value="">Select a serial port...</option>';

      if (ports.length === 0) {
        const option = document.createElement('option');
        option.value = '';
        option.textContent = 'No serial ports found';
        option.disabled = true;
        select.appendChild(option);
      } else {
        for (const port of ports) {
          const option = document.createElement('option');
          option.value = port.path;
          option.textContent = port.name;
          select.appendChild(option);
        }
      }

      if ([...select.options].some(o => o.value === currentValue)) {
        select.value = currentValue;
      }
    }
  } catch (e) {
    console.error('Failed to list serial ports:', e);
  }
}

async function refreshNetworkInterfaces() {
  if (!window.clasp) return;

  try {
    const interfaces = await window.clasp.listNetworkInterfaces();

    // Update OSC bind select
    const oscSelect = $('osc-bind-select');
    if (oscSelect) {
      const currentValue = oscSelect.value;
      oscSelect.innerHTML = '';
      for (const iface of interfaces) {
        const option = document.createElement('option');
        option.value = iface.address;
        option.textContent = iface.label;
        oscSelect.appendChild(option);
      }
      if ([...oscSelect.options].some(o => o.value === currentValue)) {
        oscSelect.value = currentValue;
      }
    }
  } catch (e) {
    console.error('Failed to list network interfaces:', e);
  }
}

// Test connection handlers
async function testDmxConnection() {
  const portPath = $('dmx-port-select')?.value;
  const resultEl = $('dmx-test-result');

  if (!portPath) {
    if (resultEl) {
      resultEl.textContent = 'Select a port first';
      resultEl.className = 'form-hint test-result error';
    }
    return;
  }

  if (resultEl) {
    resultEl.textContent = 'Testing...';
    resultEl.className = 'form-hint test-result testing';
  }

  try {
    const result = await window.clasp.testSerialPort(portPath);
    if (resultEl) {
      if (result.success) {
        resultEl.textContent = 'Connection OK';
        resultEl.className = 'form-hint test-result success';
      } else {
        resultEl.textContent = result.error || 'Connection failed';
        resultEl.className = 'form-hint test-result error';
      }
    }
  } catch (e) {
    if (resultEl) {
      resultEl.textContent = e.message;
      resultEl.className = 'form-hint test-result error';
    }
  }
}

async function testOscPort() {
  const host = $('osc-bind-select')?.value || '0.0.0.0';
  const port = parseInt($('server-osc-fields')?.querySelector('[name="oscPort"]')?.value || '9000');
  const resultEl = $('osc-test-result');

  if (resultEl) {
    resultEl.textContent = 'Testing...';
    resultEl.className = 'form-hint test-result testing';
  }

  try {
    const result = await window.clasp.testPortAvailable(host, port);
    if (resultEl) {
      if (result.success) {
        resultEl.textContent = 'Port available';
        resultEl.className = 'form-hint test-result success';
      } else {
        resultEl.textContent = result.error || 'Port in use';
        resultEl.className = 'form-hint test-result error';
      }
    }
  } catch (e) {
    if (resultEl) {
      resultEl.textContent = e.message;
      resultEl.className = 'form-hint test-result error';
    }
  }
}

// Setup hardware refresh buttons and test buttons
function setupHardwareDiscovery() {
  // Refresh buttons
  document.querySelector('.refresh-midi-ports')?.addEventListener('click', (e) => {
    e.preventDefault();
    refreshMidiPorts();
  });

  document.querySelector('.refresh-serial-ports')?.addEventListener('click', (e) => {
    e.preventDefault();
    refreshSerialPorts();
  });

  document.querySelector('.refresh-network-interfaces')?.addEventListener('click', (e) => {
    e.preventDefault();
    refreshNetworkInterfaces();
  });

  // Test buttons
  $('test-dmx-btn')?.addEventListener('click', testDmxConnection);
  $('test-osc-btn')?.addEventListener('click', testOscPort);
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
      case 'delete-router':
        deleteRouter(id);
        break;
      case 'edit-router':
        editRouter(id);
        break;
      case 'restart-router':
        restartRouter(id);
        break;
      case 'delete-server':
        deleteServer(id);
        break;
      case 'edit-server':
        editServer(id);
        break;
      case 'restart-server':
        restartServer(id);
        break;
      case 'delete-bridge':
        deleteBridge(id);
        break;
      case 'edit-bridge':
        editBridge(id);
        break;
      case 'delete-mapping':
        deleteMapping(id);
        break;
      case 'edit-mapping':
        editMapping(id);
        break;
    }
  });

  // Help button click handlers - show tooltip text as notification
  document.querySelectorAll('.help-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const helpText = btn.getAttribute('title');
      if (helpText) {
        showNotification(helpText, 'info');
      }
    });
  });

  // Mobile sidebar toggle
  const sidebarToggle = $('sidebar-toggle');
  const sidebar = document.querySelector('.sidebar');
  const sidebarBackdrop = $('sidebar-backdrop');

  function openSidebar() {
    sidebar?.classList.add('open');
    sidebarBackdrop?.classList.add('visible');
  }

  function closeSidebar() {
    sidebar?.classList.remove('open');
    sidebarBackdrop?.classList.remove('visible');
  }

  sidebarToggle?.addEventListener('click', () => {
    if (sidebar?.classList.contains('open')) {
      closeSidebar();
    } else {
      openSidebar();
    }
  });

  sidebarBackdrop?.addEventListener('click', closeSidebar);

  // Close sidebar when clicking a button inside (on mobile)
  sidebar?.addEventListener('click', (e) => {
    if (e.target.closest('.btn') && window.innerWidth <= 600) {
      // Small delay to let the action complete
      setTimeout(closeSidebar, 100);
    }
  });

  // Handle window resize - close sidebar if resizing to larger screen
  window.addEventListener('resize', () => {
    if (window.innerWidth > 600) {
      closeSidebar();
    }
  });

  // Scan button
  $('scan-btn')?.addEventListener('click', handleScan);

  // Add router button
  $('add-router-btn')?.addEventListener('click', () => {
    state.editingRouter = null;
    state.editingServer = null;
    const form = $('server-form');
    if (form) {
      form.reset();
      const serverTypeSelect = $('server-type');
      if (serverTypeSelect) {
        serverTypeSelect.value = 'clasp';
        // Hide server type dropdown for router creation (it's always CLASP)
        serverTypeSelect.closest('.form-group')?.classList.add('hidden');
      }
      updateServerTypeFields('clasp');
    }
    const modalTitle = document.querySelector('#server-modal .modal-title');
    if (modalTitle) modalTitle.textContent = 'ADD CLASP ROUTER';
    $('server-modal')?.showModal();
  });

  // Add protocol connection button
  $('add-server-btn')?.addEventListener('click', () => {
    state.editingServer = null;
    state.editingRouter = null;
    const form = $('server-form');
    if (form) {
      form.reset();
      const serverTypeSelect = $('server-type');
      if (serverTypeSelect) {
        serverTypeSelect.value = 'osc'; // Default to OSC for protocol connections
        // Show server type dropdown for protocol connections
        serverTypeSelect.closest('.form-group')?.classList.remove('hidden');
      }
      updateServerTypeFields('osc');
      populateRouterDropdowns(); // Populate router dropdowns
    }
    const modalTitle = document.querySelector('#server-modal .modal-title');
    if (modalTitle) modalTitle.textContent = 'ADD PROTOCOL CONNECTION';
    $('server-modal')?.showModal();
  });

  // Server form
  $('server-form')?.addEventListener('submit', handleAddServer);

  // Router list actions (edit/delete/restart)
  $('router-list')?.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action]');
    if (!btn) return;
    const action = btn.dataset.action;
    const id = btn.dataset.id;
    if (action === 'edit-router') editRouter(id);
    if (action === 'delete-router') deleteRouter(id);
    if (action === 'restart-router') restartRouter(id);
  });

  // Server list actions (edit/delete/restart)
  $('server-list')?.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action]');
    if (!btn) return;
    const action = btn.dataset.action;
    const id = btn.dataset.id;
    if (action === 'edit-server') editServer(id);
    if (action === 'delete-server') deleteServer(id);
    if (action === 'restart-server') restartServer(id);
  });

  // Discovered device list - click to add as connection
  $('device-list')?.addEventListener('click', (e) => {
    const item = e.target.closest('.device-item');
    if (!item) return;
    const deviceId = item.dataset.id;
    const device = state.devices.find(d => d.id === deviceId);
    if (!device) return;
    
    // Open the server modal pre-filled with this device's info
    addDiscoveredDeviceAsConnection(device);
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
  $('monitor-protocol-filter')?.addEventListener('change', (e) => {
    state.protocolFilter = e.target.value;
    renderSignalMonitor();
  });
  $('monitor-max-signals')?.addEventListener('change', (e) => {
    state.maxSignals = parseInt(e.target.value, 10);
    localStorage.setItem('clasp-max-signals', state.maxSignals);
    // Trim existing signals if needed
    if (state.signals.length > state.maxSignals) {
      state.signals = state.signals.slice(0, state.maxSignals);
      renderSignalMonitor();
    }
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
        // Auto-forward through bridges
        forwardThroughBridges(signal);
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

    // Server status updates (handles both routers and protocol connections)
    window.clasp.onServerStatus?.((status) => {
      // Check if this is a router
      const router = state.routers.find(r => r.id === status.id);
      if (router) {
        router.status = status.status;
        if (status.error) {
          router.error = status.error;
          showNotification(`Router error: ${status.error}`, 'error');
        }
        if (status.status === 'running') {
          showNotification(`Router started successfully`, 'success');
        }
        renderRouters();
        populateRouterDropdowns(); // Refresh dropdowns in case router status changed
        updateStatus();
        return;
      }

      // Check if this is a protocol connection
      const server = state.servers.find(s => s.id === status.id);
      if (server) {
        server.status = status.status;
        if (status.error) {
          server.error = status.error;
          showNotification(`Connection error: ${status.error}`, 'error');
        }
        if (status.status === 'running') {
          showNotification(`Connection started successfully`, 'success');
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
    if (window.clasp && window.clasp.scanNetwork) {
      const devices = await window.clasp.scanNetwork();
      console.log('Scan completed, found devices:', devices?.length || 0);
    } else {
      console.warn('CLASP API not available, using mock scan');
      await new Promise(r => setTimeout(r, 1500));
    }
  } catch (err) {
    console.error('Scan error:', err);
    showNotification(`Scan failed: ${err.message}`, 'error');
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

async function handleAddRouter(e) {
  e.preventDefault();
  const form = e.target;
  const data = new FormData(form);
  const isEditing = state.editingRouter !== null;

  let routerConfig = {
    id: isEditing ? state.editingRouter.id : Date.now().toString(),
    type: 'clasp',
    protocol: 'clasp',
    status: 'starting',
    address: data.get('claspAddress') || 'localhost:7330',
    name: data.get('claspName') || `CLASP Router @ ${data.get('claspAddress') || 'localhost:7330'}`,
    announce: data.get('claspAnnounce') === 'on',
    authEnabled: data.get('claspAuthEnabled') === 'on',
  };

  if (routerConfig.authEnabled && state.tokens.length > 0) {
    routerConfig.tokenFileContent = getTokenFileContent();
    routerConfig.token = state.tokens[0].token;
  } else {
    routerConfig.authEnabled = false;
    routerConfig.token = '';
  }

  try {
    if (window.clasp) {
      const result = await window.clasp.startServer(routerConfig);
      routerConfig.id = result?.id || routerConfig.id;
      routerConfig.status = 'connected';
    } else {
      routerConfig.status = 'connected';
    }

    if (isEditing) {
      const idx = state.routers.findIndex(r => r.id === routerConfig.id);
      if (idx !== -1) {
        state.routers[idx] = routerConfig;
      } else {
        state.routers.push(routerConfig);
      }
    } else {
      state.routers.push(routerConfig);
    }

    state.editingRouter = null;
    saveRoutersToStorage();
    renderRouters();
    populateRouterDropdowns(); // Refresh router dropdowns for protocol connections
    renderServers(); // Re-render protocol connections to update connection status
    renderFlowDiagram(); // Update flow diagram when routers change
    updateStatus();
    $('server-modal')?.close();
    form.reset();
    // Show server type dropdown again when modal closes (for next use)
    const serverTypeSelect = $('server-type');
    if (serverTypeSelect) {
      serverTypeSelect.closest('.form-group')?.classList.remove('hidden');
    }
    updateServerTypeFields('clasp');
  } catch (err) {
    console.error('Failed to start router:', err);
    routerConfig.status = 'error';
    routerConfig.error = err.message;

    if (isEditing) {
      const idx = state.routers.findIndex(r => r.id === routerConfig.id);
      if (idx !== -1) {
        state.routers[idx] = routerConfig;
      } else {
        state.routers.push(routerConfig);
      }
    } else {
      state.routers.push(routerConfig);
    }

    state.editingRouter = null;
    saveRoutersToStorage();
    renderRouters();
  }
}

async function handleAddServer(e) {
  e.preventDefault();
  const form = e.target;
  const data = new FormData(form);
  const serverType = data.get('serverType') || 'clasp';
  const isEditing = state.editingServer !== null;

  // Separate routers from protocol connections
  if (serverType === 'clasp') {
    // This is a CLASP router
    return handleAddRouter(e);
  }

  // This is a protocol connection
  // Validate router availability before proceeding
  const requestedRouterId = data.get('routerId') || null;
  let targetRouter = null;
  
  if (requestedRouterId) {
    targetRouter = state.routers.find(r => r.id === requestedRouterId);
  } else {
    // Auto-select: find first available router
    targetRouter = state.routers.find(r => 
      r.isRemote || r.status === 'running' || r.status === 'connected' || r.status === 'available'
    );
  }
  
  if (!targetRouter) {
    showNotification('No CLASP router available. Please add a router first.', 'error');
    // Optionally open router creation modal
    const shouldCreateRouter = confirm('No router found. Would you like to create one now?');
    if (shouldCreateRouter) {
      $('add-router-btn')?.click();
    }
    return;
  }
  
  let serverConfig = {
    id: isEditing ? state.editingServer.id : Date.now().toString(),
    type: serverType,
    protocol: serverType,
    status: 'starting',
    routerId: targetRouter.id, // Use found router ID
  };

  // Build config based on server type
  switch (serverType) {
    case 'osc':
      const oscIp = data.get('oscIp')?.trim();
      serverConfig.bind = oscIp || data.get('oscBind') || '0.0.0.0';
      serverConfig.port = parseInt(data.get('oscPort')) || 9000;
      serverConfig.address = `${serverConfig.bind}:${serverConfig.port}`;
      serverConfig.name = `OSC Server @ ${serverConfig.address}`;
      break;

    case 'mqtt':
      serverConfig.host = data.get('mqttHost') || 'localhost';
      serverConfig.port = parseInt(data.get('mqttPort')) || 1883;
      serverConfig.clientId = data.get('mqttClientId') || '';
      serverConfig.topics = (data.get('mqttTopics') || '#').split(',').map(t => t.trim());
      serverConfig.qos = parseInt(data.get('mqttQos')) || 0;
      serverConfig.keepAlive = parseInt(data.get('mqttKeepAlive')) || 60;
      serverConfig.namespace = data.get('mqttNamespace') || '/mqtt';
      // Authentication
      serverConfig.authEnabled = data.get('mqttAuthEnabled') === 'on';
      if (serverConfig.authEnabled) {
        serverConfig.username = data.get('mqttUsername') || '';
        serverConfig.password = data.get('mqttPassword') || '';
      }
      serverConfig.address = `${serverConfig.host}:${serverConfig.port}`;
      serverConfig.name = `MQTT @ ${serverConfig.address}`;
      break;

    case 'websocket':
      serverConfig.mode = data.get('wsMode') || 'server';
      serverConfig.address = data.get('wsAddress') || '0.0.0.0:8080';
      serverConfig.format = data.get('wsFormat') || 'json';
      serverConfig.pingInterval = parseInt(data.get('wsPingInterval')) || 30;
      serverConfig.namespace = data.get('wsNamespace') || '/websocket';
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

    case 'sacn':
      serverConfig.mode = data.get('sacnMode') || 'receiver';
      serverConfig.universes = (data.get('sacnUniverses') || '1').split(',').map(u => parseInt(u.trim())).filter(u => u > 0 && u < 64000);
      serverConfig.sourceName = data.get('sacnSourceName') || 'CLASP sACN Bridge';
      serverConfig.priority = parseInt(data.get('sacnPriority')) || 100;
      serverConfig.multicast = data.get('sacnMulticast') === 'on';
      serverConfig.bindAddress = data.get('sacnBindAddress') || '';
      if (!serverConfig.multicast) {
        serverConfig.unicastDestinations = (data.get('sacnUnicastDests') || '').split(',').map(d => d.trim()).filter(Boolean);
      }
      serverConfig.address = `sACN ${serverConfig.mode} (U: ${serverConfig.universes.join(',')})`;
      serverConfig.name = `sACN ${serverConfig.mode.charAt(0).toUpperCase() + serverConfig.mode.slice(1)} - Universes ${serverConfig.universes.join(', ')}`;
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
    renderFlowDiagram(); // Update flow diagram when servers change
    updateStatus();
    $('server-modal')?.close();
    form.reset();
    updateServerTypeFields('osc'); // Reset to default for protocol connections
    const modalTitle = document.querySelector('#server-modal .modal-title');
    if (modalTitle) modalTitle.textContent = 'ADD PROTOCOL CONNECTION';
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

async function deleteRouter(id) {
  try {
    const router = state.routers.find(r => r.id === id);
    
    // For local routers, stop the server process
    // For remote routers, just remove from list (nothing to stop)
    if (router && !router.isRemote && window.clasp) {
      await window.clasp.stopServer(id);
    }
    
    state.routers = state.routers.filter(r => r.id !== id);
    saveRoutersToStorage();
    renderRouters();
    populateRouterDropdowns(); // Refresh dropdowns since router was removed
    renderServers(); // Re-render protocol connections to update connection status
    renderFlowDiagram(); // Update flow diagram
    updateStatus();
    
    if (router?.isRemote) {
      showNotification(`Removed remote router: ${router.name}`, 'info');
    }
  } catch (err) {
    console.error('Failed to delete router:', err);
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
    renderFlowDiagram(); // Update flow diagram
    updateStatus();
  } catch (err) {
    console.error('Failed to delete server:', err);
  }
}

async function restartRouter(id) {
  const router = state.routers.find(r => r.id === id);
  if (!router) return;

  router.status = 'reconnecting';
  router.error = null;
  renderRouters();
  showNotification(`Restarting ${router.name}...`, 'info');
  
  try {
    if (window.clasp) {
      await window.clasp.stopServer(id);
      await new Promise(resolve => setTimeout(resolve, 500));
      const result = await window.clasp.startServer(router);
      router.id = result?.id || router.id;
      router.status = 'connected';
    } else {
      router.status = 'connected';
    }
    saveRoutersToStorage();
    renderRouters();
    populateRouterDropdowns(); // Refresh dropdowns in case router status changed
    renderServers(); // Re-render protocol connections to update connection status
    renderFlowDiagram(); // Update flow diagram
    showNotification(`${router.name} restarted`, 'success');
  } catch (err) {
    console.error('Failed to restart router:', err);
    router.status = 'error';
    router.error = err.message;
    saveRoutersToStorage();
    renderRouters();
    populateRouterDropdowns(); // Refresh dropdowns
    renderServers(); // Re-render protocol connections
    renderFlowDiagram(); // Update flow diagram
    showNotification(`Failed to restart: ${err.message}`, 'error');
  }
}

function editRouter(id) {
  const router = state.routers.find(r => r.id === id);
  if (!router) return;

  state.editingRouter = router;
  state.editingServer = null;

  // Populate form
  const form = $('server-form');
  if (form) {
    form.reset();
    const serverTypeSelect = $('server-type');
    if (serverTypeSelect) {
      serverTypeSelect.value = 'clasp';
      // Hide server type dropdown when editing router (it's always CLASP)
      serverTypeSelect.closest('.form-group')?.classList.add('hidden');
    }
    updateServerTypeFields('clasp');

    // Fill in router fields
    const claspAddressInput = $('claspAddress');
    if (claspAddressInput) claspAddressInput.value = router.address || 'localhost:7330';
    
    const claspNameInput = $('claspName');
    if (claspNameInput) claspNameInput.value = router.name || 'CLASP Router';

    const claspAnnounceInput = $('claspAnnounce');
    if (claspAnnounceInput) claspAnnounceInput.checked = router.announce !== false;

    const claspAuthEnabledInput = $('claspAuthEnabled');
    if (claspAuthEnabledInput) claspAuthEnabledInput.checked = router.authEnabled === true;

    $('server-modal')?.showModal();
  }
}

async function restartServer(id) {
  const server = state.servers.find(s => s.id === id);
  if (!server) return;

  // Update status to show restarting
  server.status = 'reconnecting';
  server.error = null;
  renderServers();
  showNotification(`Restarting ${server.name}...`, 'info');

  try {
    // Stop the server first
    if (window.clasp) {
      await window.clasp.stopServer(id);
    }

    // Small delay before restart
    await new Promise(resolve => setTimeout(resolve, 500));

    // Restart with the same config
    if (window.clasp) {
      const result = await window.clasp.startServer(server);
      server.id = result?.id || server.id;
      server.status = 'connected';
      server.error = null;
      showNotification(`${server.name} restarted successfully`, 'success');
    } else {
      server.status = 'connected';
    }
  } catch (err) {
    console.error('Failed to restart server:', err);
    server.status = 'error';
    server.error = err.message || 'Restart failed';
    showNotification(`Failed to restart ${server.name}: ${err.message}`, 'error');
  }

  saveServersToStorage();
  renderServers();
  updateStatus();
}

// Add a discovered CLASP server as a remote router endpoint
function addDiscoveredDeviceAsConnection(device) {
  // Build the address
  const address = device.address || device.host;
  const port = device.port || 7330;
  const fullAddress = address ? (address.includes(':') ? address : `${address}:${port}`) : 'localhost:7330';
  const wsAddress = fullAddress.startsWith('ws://') ? fullAddress : `ws://${fullAddress}`;
  
  // Check if we already have this router saved
  const existingRouter = state.routers.find(r => 
    r.address === fullAddress || r.address === wsAddress || r.remoteAddress === fullAddress
  );
  
  if (existingRouter) {
    showNotification(`Router "${existingRouter.name}" already added`, 'warning');
    return;
  }
  
  // Create a remote router entry (not a local one we start)
  const remoteRouter = {
    id: `remote-${Date.now()}`,
    type: 'clasp',
    protocol: 'clasp',
    name: device.name || `Remote Router @ ${fullAddress}`,
    address: wsAddress,
    remoteAddress: fullAddress,
    isRemote: true, // Flag to indicate this is a remote router we connect TO
    status: 'available', // Remote routers are "available" not "running"
    discoveredFrom: device.id,
  };
  
  // Add to routers list
  state.routers.push(remoteRouter);
  saveRoutersToStorage();
  renderRouters();
  
  // Refresh router dropdowns in protocol connection forms
  populateRouterDropdowns();
  
  showNotification(`Added remote router: ${remoteRouter.name}. You can now route protocol connections to it.`, 'success');
}

function editServer(id) {
  const server = state.servers.find(s => s.id === id);
  if (!server) return;

  state.editingServer = server;

  // Update modal title based on type
  const modalTitle = document.querySelector('#server-modal .modal-title');
  if (modalTitle) {
    if (server.type === 'clasp' || server.protocol === 'clasp') {
      modalTitle.textContent = 'EDIT CLASP ROUTER';
    } else {
      modalTitle.textContent = 'EDIT PROTOCOL CONNECTION';
    }
  }

  // Set server type
  const typeSelect = $('server-type');
  if (typeSelect) {
    typeSelect.value = server.protocol || server.type || 'clasp';
    // Show/hide server type dropdown based on whether it's a router or protocol connection
    if (server.type === 'clasp' || server.protocol === 'clasp') {
      // Hide for routers
      typeSelect.closest('.form-group')?.classList.add('hidden');
    } else {
      // Show for protocol connections
      typeSelect.closest('.form-group')?.classList.remove('hidden');
    }
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
      if (form.elements.oscIp) {
        // If bind is a specific IP (not 0.0.0.0), put it in the IP field
        form.elements.oscIp.value = (server.bind && server.bind !== '0.0.0.0') ? server.bind : '';
      }
      form.elements.oscPort.value = server.port || 9000;
      const oscRouterSelect = $('osc-router-select');
      if (oscRouterSelect) {
        populateRouterDropdowns(); // Populate first
        // Use connected router ID if available, otherwise use assigned router ID
        oscRouterSelect.value = server.connectedRouterId || server.routerId || '';
      }
      break;
    case 'mqtt':
      form.elements.mqttHost.value = server.host || 'localhost';
      form.elements.mqttPort.value = server.port || 1883;
      if (form.elements.mqttClientId) form.elements.mqttClientId.value = server.clientId || '';
      form.elements.mqttTopics.value = (server.topics || ['#']).join(', ');
      if (form.elements.mqttQos) form.elements.mqttQos.value = server.qos || 0;
      if (form.elements.mqttKeepAlive) form.elements.mqttKeepAlive.value = server.keepAlive || 60;
      if (form.elements.mqttNamespace) form.elements.mqttNamespace.value = server.namespace || '/mqtt';
      // Auth fields
      if (form.elements.mqttAuthEnabled) {
        form.elements.mqttAuthEnabled.checked = server.authEnabled || false;
        const authFields = $('mqtt-auth-fields');
        if (authFields) authFields.classList.toggle('hidden', !server.authEnabled);
      }
      if (form.elements.mqttUsername) form.elements.mqttUsername.value = server.username || '';
      if (form.elements.mqttPassword) form.elements.mqttPassword.value = server.password || '';
      if (form.elements.routerId) {
        const routerSelect = $('mqtt-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'websocket':
      form.elements.wsMode.value = server.mode || 'server';
      form.elements.wsAddress.value = server.address || '0.0.0.0:8080';
      if (form.elements.wsFormat) form.elements.wsFormat.value = server.format || 'json';
      if (form.elements.wsPingInterval) form.elements.wsPingInterval.value = server.pingInterval || 30;
      if (form.elements.wsNamespace) form.elements.wsNamespace.value = server.namespace || '/websocket';
      if (form.elements.routerId) {
        const routerSelect = $('websocket-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'http':
      form.elements.httpBind.value = server.bind || '0.0.0.0:3000';
      form.elements.httpBasePath.value = server.basePath || '/api';
      form.elements.httpCors.checked = server.cors !== false;
      if (form.elements.routerId) {
        const routerSelect = $('http-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'artnet':
      form.elements.artnetBind.value = server.bind || '0.0.0.0:6454';
      form.elements.artnetSubnet.value = server.subnet || 0;
      form.elements.artnetUniverse.value = server.universe || 0;
      if (form.elements.routerId) {
        const routerSelect = $('artnet-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'sacn':
      if (form.elements.sacnMode) form.elements.sacnMode.value = server.mode || 'receiver';
      if (form.elements.sacnUniverses) form.elements.sacnUniverses.value = (server.universes || [1]).join(', ');
      if (form.elements.sacnSourceName) form.elements.sacnSourceName.value = server.sourceName || 'CLASP sACN Bridge';
      if (form.elements.sacnPriority) form.elements.sacnPriority.value = server.priority || 100;
      if (form.elements.sacnMulticast) {
        form.elements.sacnMulticast.checked = server.multicast !== false;
        const unicastFields = $('sacn-unicast-fields');
        if (unicastFields) unicastFields.classList.toggle('hidden', server.multicast !== false);
      }
      if (form.elements.sacnUnicastDests) form.elements.sacnUnicastDests.value = (server.unicastDestinations || []).join(', ');
      if (form.elements.sacnBindAddress) form.elements.sacnBindAddress.value = server.bindAddress || '';
      if (form.elements.routerId) {
        const routerSelect = $('sacn-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'dmx':
      form.elements.dmxPort.value = server.serialPort || '/dev/ttyUSB0';
      form.elements.dmxUniverse.value = server.universe || 0;
      if (form.elements.routerId) {
        const routerSelect = $('dmx-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'midi':
      if (form.elements.midiInput) form.elements.midiInput.value = server.inputPort || '';
      if (form.elements.midiOutput) form.elements.midiOutput.value = server.outputPort || '';
      if (form.elements.routerId) {
        const routerSelect = $('midi-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
    case 'socketio':
      if (form.elements.socketioMode) form.elements.socketioMode.value = server.mode || 'server';
      if (form.elements.socketioAddress) form.elements.socketioAddress.value = server.address || '0.0.0.0:3001';
      if (form.elements.socketioNamespace) form.elements.socketioNamespace.value = server.namespace || '/';
      if (form.elements.routerId) {
        const routerSelect = $('socketio-router-select');
        if (routerSelect) {
          populateRouterDropdowns(); // Populate first
          routerSelect.value = server.connectedRouterId || server.routerId || '';
      }
      } else {
      populateRouterDropdowns();
      }
      break;
  }

  $('server-modal')?.showModal();
}

// ============================================
// Output Target Management
// ============================================



function loadMaxSignalsSetting() {
  try {
    const saved = localStorage.getItem('clasp-max-signals');
    if (saved) {
      state.maxSignals = parseInt(saved, 10);
      // Update dropdown to match saved value
      const dropdown = $('monitor-max-signals');
      if (dropdown) dropdown.value = state.maxSignals.toString();
    }
  } catch (e) {
    console.error('Failed to load max signals setting:', e);
  }
}

// ============================================
// Bridge Management
// ============================================

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
    // Check for duplicate before creating
    const existing = state.bridges.find(b => 
      b.source === config.source && 
      b.target === config.target &&
      b.sourceAddr === config.sourceAddr &&
      b.targetAddr === config.targetAddr
    );
    if (existing) {
      console.warn('Bridge already exists, skipping duplicate');
      $('bridge-modal')?.close();
      form.reset();
      return;
    }
    
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

function editMapping(id) {
  const mapping = state.mappings.find(m => m.id === id);
  if (!mapping) return;

  state.editingMapping = id;

  // Update modal title
  const modalTitle = document.querySelector('#mapping-modal .modal-title');
  if (modalTitle) modalTitle.textContent = 'EDIT MAPPING';

  const form = $('mapping-form');
  if (!form) return;

  // Set source protocol and trigger field switching
  const sourceProtocol = $('mapping-source-protocol');
  if (sourceProtocol) {
    sourceProtocol.value = mapping.source.protocol;
    sourceProtocol.dispatchEvent(new Event('change'));
  }

  // Populate source fields based on protocol
  setTimeout(() => {
    switch (mapping.source.protocol) {
      case 'clasp':
        if (form.elements.sourceClaspAddress) form.elements.sourceClaspAddress.value = mapping.source.address || '';
        break;
      case 'osc':
        if (form.elements.sourceOscAddress) form.elements.sourceOscAddress.value = mapping.source.address || '';
        break;
      case 'midi':
        if (form.elements.sourceMidiChannel) form.elements.sourceMidiChannel.value = mapping.source.midiChannel || '*';
        if (form.elements.sourceMidiType) form.elements.sourceMidiType.value = mapping.source.midiType || 'note';
        if (form.elements.sourceMidiNumber) form.elements.sourceMidiNumber.value = mapping.source.midiNumber ?? '';
        break;
      case 'dmx':
      case 'artnet':
        if (form.elements.sourceDmxUniverse) form.elements.sourceDmxUniverse.value = mapping.source.dmxUniverse ?? 0;
        if (form.elements.sourceDmxChannel) form.elements.sourceDmxChannel.value = mapping.source.dmxChannel ?? 1;
        break;
    }

    // Set transform
    const transformSelect = $('mapping-transform');
    if (transformSelect) {
      transformSelect.value = mapping.transform?.type || 'direct';
      transformSelect.dispatchEvent(new Event('change'));
    }

    // Populate transform params
    if (mapping.transform) {
      const t = mapping.transform;
      if (form.elements.scaleInMin) form.elements.scaleInMin.value = t.scaleInMin ?? 0;
      if (form.elements.scaleInMax) form.elements.scaleInMax.value = t.scaleInMax ?? 1;
      if (form.elements.scaleOutMin) form.elements.scaleOutMin.value = t.scaleOutMin ?? 0;
      if (form.elements.scaleOutMax) form.elements.scaleOutMax.value = t.scaleOutMax ?? 1;
      if (form.elements.clampMin) form.elements.clampMin.value = t.clampMin ?? 0;
      if (form.elements.clampMax) form.elements.clampMax.value = t.clampMax ?? 1;
      if (form.elements.threshold) form.elements.threshold.value = t.threshold ?? 0.5;
      if (form.elements.expression) form.elements.expression.value = t.expression || '';
      if (form.elements.javascriptCode) form.elements.javascriptCode.value = t.javascriptCode || '';
    }

    // Set target protocol and trigger field switching
    const targetProtocol = $('mapping-target-protocol');
    if (targetProtocol) {
      targetProtocol.value = mapping.target.protocol;
      targetProtocol.dispatchEvent(new Event('change'));
    }

    // Populate target fields
    setTimeout(() => {
      switch (mapping.target.protocol) {
        case 'clasp':
          if (form.elements.targetClaspAddress) form.elements.targetClaspAddress.value = mapping.target.address || '';
          break;
        case 'osc':
          if (form.elements.targetOscAddress) form.elements.targetOscAddress.value = mapping.target.address || '';
          break;
        case 'midi':
          if (form.elements.targetMidiChannel) form.elements.targetMidiChannel.value = mapping.target.midiChannel || 1;
          if (form.elements.targetMidiType) form.elements.targetMidiType.value = mapping.target.midiType || 'note';
          if (form.elements.targetMidiNumber) form.elements.targetMidiNumber.value = mapping.target.midiNumber ?? 60;
          break;
        case 'dmx':
        case 'artnet':
          if (form.elements.targetDmxUniverse) form.elements.targetDmxUniverse.value = mapping.target.dmxUniverse ?? 0;
          if (form.elements.targetDmxChannel) form.elements.targetDmxChannel.value = mapping.target.dmxChannel ?? 1;
          break;
      }
    }, 50);
  }, 50);

  $('mapping-modal')?.showModal();
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

function editBridge(id) {
  const bridge = state.bridges.find(b => b.id === id);
  if (!bridge) return;

  // Update modal title
  const modalTitle = document.querySelector('#bridge-modal .modal-title');
  if (modalTitle) modalTitle.textContent = 'EDIT BRIDGE';

  const form = $('bridge-form');
  if (!form) return;

  // Store the bridge ID for update instead of create
  form.dataset.editId = id;

  // Populate form fields
  if (form.elements.bridgeSource) form.elements.bridgeSource.value = bridge.source || '';
  if (form.elements.bridgeSourceAddr) form.elements.bridgeSourceAddr.value = bridge.sourceAddr || '';
  if (form.elements.bridgeTarget) form.elements.bridgeTarget.value = bridge.target || '';
  if (form.elements.bridgeTargetAddr) form.elements.bridgeTargetAddr.value = bridge.targetAddr || '';

  $('bridge-modal')?.showModal();
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

  // Enrich signal with protocol if not provided
  const enrichedSignal = {
    ...signal,
    timestamp: Date.now(),
    // Use provided protocol or detect from signal content
    protocol: signal.protocol || detectProtocol(signal),
  };

  state.signals.unshift(enrichedSignal);

  if (state.signals.length > state.maxSignals) {
    state.signals = state.signals.slice(0, state.maxSignals);
  }

  // Track signal history for sparklines
  addSignalToHistory(enrichedSignal);

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

// Auto-forward signals through configured bridges
function forwardThroughBridges(signal) {
  const signalProtocol = signal.protocol || detectProtocol(signal);

  for (const bridge of state.bridges) {
    if (!bridge.active) continue;

    // Check if this signal came from this bridge's source protocol
    if (bridge.source !== signalProtocol) continue;

    // Don't forward if source and target are the same
    if (bridge.source === bridge.target) continue;

    // Forward to target
    const targetAddress = signal.address || signal.topic || '/forwarded';
    const value = signal.value;

    // Send via appropriate output
    forwardSignalToTarget(bridge, targetAddress, value, signal);
  }
}

async function forwardSignalToTarget(bridge, address, value, originalSignal) {
  // Add forwarded signal to monitor so user sees it
  const forwardedSignal = {
    address,
    value,
    protocol: bridge.target,
    serverName: `→ ${bridge.target.toUpperCase()}`,
    bridgeId: bridge.id,
    forwarded: true,
    originalProtocol: bridge.source,
  };

  // Add to monitor (will show as target protocol)
  signalCount++;
  state.signals.unshift({
    ...forwardedSignal,
    timestamp: Date.now(),
  });
  if (state.signals.length > state.maxSignals) {
    state.signals = state.signals.slice(0, state.maxSignals);
  }

  // Actually send to target via IPC
  if (window.clasp?.sendSignal && bridge.targetAddr) {
    try {
      await window.clasp.sendSignal({
        bridgeId: bridge.id,
        address,
        value,
        targetProtocol: bridge.target,
        targetAddr: bridge.targetAddr,
      });
    } catch (err) {
      console.error(`Failed to forward to ${bridge.target}:`, err);
    }
  }

  renderSignalMonitor();
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
      const range = transform.scaleInMax - transform.scaleInMin;
      if (range === 0) return transform.scaleOutMin; // Avoid division by zero
      const normalized = (value - transform.scaleInMin) / range;
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

    case 'trigger':
      // Trigger outputs 1 when any value is received (non-zero), used for one-shot events
      return value !== 0 ? 1 : 0;

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

function renderRouters() {
  const list = $('router-list');
  if (!list) return;

  if (state.routers.length === 0) {
    list.innerHTML = `
      <div class="empty-state-small">
        <span class="empty-state-text">No routers configured</span>
      </div>
    `;
  } else {
    list.innerHTML = state.routers.map(router => {
      const isRemote = router.isRemote === true;
      const statusClass = isRemote ? 'available' : getServerStatusClass(router.status);
      const statusTitle = isRemote ? `Remote router at ${router.remoteAddress || router.address}` : getServerStatusTitle(router);
      const hasError = !isRemote && (router.status === 'error' || router.error);
      const badgeClass = isRemote ? 'remote' : 'clasp';
      const badgeText = isRemote ? '↗ REMOTE' : 'CLASP';
      
      return `
      <div class="device-item ${hasError ? 'device-item-error' : ''} ${isRemote ? 'device-item-remote' : ''}" data-id="${router.id}" title="${statusTitle}">
        <div class="device-item-main">
          <span class="status-dot ${statusClass}" title="${statusTitle}"></span>
          <span class="device-protocol-badge ${badgeClass}">${badgeText}</span>
          <span class="device-name">${router.name}</span>
        </div>
        ${isRemote ? `<div class="device-connection-info">${router.remoteAddress || router.address}</div>` : ''}
        ${hasError ? `<div class="device-error-msg">${escapeHtml(router.error || 'Connection error')}</div>` : ''}
        <div class="device-actions">
          ${hasError && !isRemote ? `
          <button class="btn-device-restart" data-action="restart-router" data-id="${router.id}" title="Restart router">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 11-2.52-6.24"/><path d="M21 3v6h-6"/></svg>
          </button>
          ` : ''}
          ${!isRemote ? `
          <button class="btn-device-edit" data-action="edit-router" data-id="${router.id}" title="Edit router">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          ` : ''}
          <button class="btn-device-delete" data-action="delete-router" data-id="${router.id}" title="${isRemote ? 'Remove remote router' : 'Stop router'}">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </div>
    `;
    }).join('');
  }

  // Update badge if exists
  const badge = $('router-badge');
  if (badge) badge.textContent = state.routers.length;
}

function renderServers() {
  const list = $('server-list');
  if (!list) return;

  if (state.servers.length === 0) {
    list.innerHTML = `
      <div class="empty-state-small">
        <span class="empty-state-text">No protocol connections</span>
      </div>
    `;
  } else {
    list.innerHTML = state.servers.map(server => {
      const statusClass = getServerStatusClass(server.status);
      const statusTitle = getServerStatusTitle(server);
      const hasError = server.status === 'error' || server.error;
      const serverType = server.type || server.protocol || 'clasp';
      
      // Find connected router
      let routerInfo = null;
      if (server.routerId) {
        const router = state.routers.find(r => r.id === server.routerId);
        if (router) {
          routerInfo = router;
        }
      } else if (server.connectedRouterId) {
        // Use actually connected router ID if available
        const router = state.routers.find(r => r.id === server.connectedRouterId);
        if (router) {
          routerInfo = router;
        }
      } else {
        // Auto-selected - find first running router
        const runningRouter = state.routers.find(r => r.status === 'running' || r.status === 'connected');
        if (runningRouter) {
          routerInfo = runningRouter;
        }
      }
      
      // Check actual router connection status
      const isRouterConnected = server.routerConnected === true;
      const routerConnectionError = server.routerError;
      const routerConnectionStatus = isRouterConnected ? 'connected' : (routerConnectionError ? 'error' : 'disconnected');
      
      return `
      <div class="device-item ${hasError ? 'device-item-error' : ''}" data-id="${server.id}" title="${statusTitle}">
        <div class="device-item-main">
          <span class="status-dot ${statusClass}" title="${statusTitle}"></span>
          <span class="device-protocol-badge ${serverType}">${protocolNames[serverType] || serverType.toUpperCase()}</span>
          <span class="device-name">${server.name}</span>
        </div>
        ${routerInfo ? `
          <div class="device-connection-info">
            → Router: ${routerInfo.name}
            ${isRouterConnected ? '<span class="router-status-badge connected" title="Connected to router">●</span>' : ''}
            ${routerConnectionError ? '<span class="router-status-badge error" title="Router connection failed">⚠</span>' : ''}
            ${!isRouterConnected && !routerConnectionError && routerInfo ? '<span class="router-status-badge disconnected" title="Not connected to router">○</span>' : ''}
          </div>
        ` : ''}
        ${routerConnectionError ? `<div class="device-error-msg">Router connection: ${escapeHtml(routerConnectionError)}</div>` : ''}
        ${hasError ? `<div class="device-error-msg">${escapeHtml(server.error || 'Connection error')}</div>` : ''}
        <div class="device-actions">
          ${hasError ? `
          <button class="btn-device-restart" data-action="restart-server" data-id="${server.id}" title="Restart connection">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 11-2.52-6.24"/><path d="M21 3v6h-6"/></svg>
          </button>
          ` : ''}
          <button class="btn-device-edit" data-action="edit-server" data-id="${server.id}" title="Edit connection">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
          </button>
          <button class="btn-device-delete" data-action="delete-server" data-id="${server.id}" title="Stop connection">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </div>
    `;
    }).join('');
  }

  // Update badge
  const badge = $('server-badge');
  if (badge) badge.textContent = state.servers.length;
}

function getServerStatusClass(status) {
  switch (status) {
    case 'connected':
    case 'running':
      return 'connected';
    case 'starting':
    case 'reconnecting':
      return 'connecting';
    case 'error':
    case 'disconnected':
      return 'error';
    default:
      return 'available';
  }
}

function getServerStatusTitle(server) {
  if (server.error) return `Error: ${server.error}`;
  switch (server.status) {
    case 'connected':
    case 'running':
      return 'Running';
    case 'starting':
      return 'Starting...';
    case 'reconnecting':
      return 'Reconnecting...';
    case 'error':
      return 'Error';
    case 'disconnected':
      return 'Disconnected';
    default:
      return 'Unknown';
  }
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

  list.innerHTML = state.devices.map(device => {
    const address = device.address || device.host || '';
    const port = device.port || 7330;
    const displayAddress = address ? (address.includes(':') ? address : `${address}:${port}`) : '';
    const tooltip = `Click to add as connection${displayAddress ? ` (${displayAddress})` : ''}`;
    
    return `
    <div class="device-item device-item-clickable" data-id="${device.id}" title="${tooltip}">
      <span class="status-dot ${device.status || 'available'}"></span>
      <span class="device-protocol-badge ${device.protocol || 'clasp'}">${protocolNames[device.protocol] || device.protocol || 'CLASP'}</span>
      <span class="device-name">${device.name}</span>
      <svg class="device-connect-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
    </div>
  `}).join('');

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
        <div class="empty-state-text">No direct connections configured</div>
        <div class="empty-state-hint">Create a direct protocol-to-protocol bridge that bypasses the CLASP router (uses CLASP format for translation)</div>
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
        <button class="btn btn-sm btn-secondary" data-action="edit-bridge" data-id="${bridge.id}" title="Edit">
          ${icons.edit}
        </button>
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
        <button class="btn btn-sm btn-secondary" data-action="edit-mapping" data-id="${mapping.id}" title="Edit">
          ${icons.edit}
        </button>
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

  // Filter signals by protocol
  let signals = state.signals;
  if (state.protocolFilter && state.protocolFilter !== 'all') {
    signals = signals.filter(s => s.protocol === state.protocolFilter);
  }

  // Filter signals by address/text
  if (state.monitorFilter) {
    signals = signals.filter(s =>
      (s.address && s.address.toLowerCase().includes(state.monitorFilter)) ||
      (s.bridgeId && s.bridgeId.toLowerCase().includes(state.monitorFilter)) ||
      (s.serverName && s.serverName.toLowerCase().includes(state.monitorFilter))
    );
  }

  const hasFilters = state.monitorFilter || (state.protocolFilter && state.protocolFilter !== 'all');

  if (signals.length === 0) {
    monitor.innerHTML = `
      <div class="signal-empty">
        <span>${hasFilters ? 'No matching signals' : 'Waiting for signals...'}</span>
      </div>
    `;
    return;
  }

  monitor.innerHTML = signals.slice(0, 100).map(s => {
    const val = typeof s.value === 'number' ? s.value : 0;
    const percent = Math.min(100, Math.max(0, Math.abs(val) * 100));
    const displayVal = formatSignalValue(s.value);

    // Protocol badge (small, inline)
    const protocolName = protocolNames[s.protocol] || s.protocol?.toUpperCase() || '?';
    const protocolClass = `protocol-${s.protocol || 'unknown'}`;

    // Check if forwarded
    const isForwarded = s.forwarded === true;
    const forwardedClass = isForwarded ? 'signal-forwarded' : '';
    const directionIcon = isForwarded ? '→' : '←';

    // Build tooltip with source info
    let tooltipParts = [];
    if (isForwarded) tooltipParts.push(`Forwarded from ${s.originalProtocol?.toUpperCase()}`);
    if (s.serverName && !isForwarded) tooltipParts.push(s.serverName);
    if (s.serverPort) tooltipParts.push(`Port: ${s.serverPort}`);
    if (s.serverAddress) tooltipParts.push(s.serverAddress);
    if (s.bridgeId) tooltipParts.push(`Bridge: ${s.bridgeId.substring(0, 12)}`);
    const tooltip = tooltipParts.join(' | ') || '';

    // Build value tooltip for complex values
    const valueTooltip = typeof s.value === 'object' && s.value !== null
      ? escapeHtml(JSON.stringify(s.value, null, 2).substring(0, 500))
      : '';

    return `
      <div class="signal-item ${forwardedClass}" title="${tooltip}">
        <span class="signal-direction">${directionIcon}</span>
        <span class="signal-protocol-badge ${protocolClass}">${protocolName}</span>
        <span class="signal-address">${s.address || s.topic || '--'}</span>
        <span class="signal-value" ${valueTooltip ? `title="${valueTooltip}"` : ''}>${displayVal}</span>
        <div class="signal-bar">
          <div class="signal-bar-fill" style="width: ${percent}%"></div>
        </div>
      </div>
    `;
  }).join('');
}

function formatSignalValue(value, maxLength = 60) {
  if (value === null) return 'null';
  if (value === undefined) return 'undefined';

  if (typeof value === 'number') {
    return value % 1 === 0 ? value.toString() : value.toFixed(3);
  }
  if (typeof value === 'boolean') {
    return value ? 'ON' : 'OFF';
  }
  if (typeof value === 'string') {
    if (value.length > maxLength) {
      return `"${value.substring(0, maxLength - 3)}..."`;
    }
    return value.length > 20 ? `"${value}"` : value;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return '[]';
    // Show first few elements
    const preview = value.slice(0, 4).map(v => formatSignalValueShort(v)).join(', ');
    const suffix = value.length > 4 ? `, +${value.length - 4}` : '';
    const result = `[${preview}${suffix}]`;
    return result.length > maxLength ? `[${value.length} items]` : result;
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value);
    if (keys.length === 0) return '{}';
    // Show first few key-value pairs
    const preview = keys.slice(0, 3).map(k => {
      const v = formatSignalValueShort(value[k]);
      return `${k}: ${v}`;
    }).join(', ');
    const suffix = keys.length > 3 ? `, +${keys.length - 3}` : '';
    const result = `{${preview}${suffix}}`;
    return result.length > maxLength ? `{${keys.length} keys}` : result;
  }
  return String(value);
}

function formatSignalValueShort(value) {
  if (value === null) return 'null';
  if (value === undefined) return '?';
  if (typeof value === 'number') return value % 1 === 0 ? value.toString() : value.toFixed(2);
  if (typeof value === 'boolean') return value ? '1' : '0';
  if (typeof value === 'string') return value.length > 12 ? `"${value.substring(0, 9)}..."` : `"${value}"`;
  if (Array.isArray(value)) return `[${value.length}]`;
  if (typeof value === 'object') return `{${Object.keys(value).length}}`;
  return String(value).substring(0, 10);
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
// Preset Picker
// ============================================

function setupPresetPicker() {
  $('presets-btn')?.addEventListener('click', () => {
    renderPresetGrid();
    $('preset-modal')?.showModal();
  });
}

function renderPresetGrid() {
  const grid = $('preset-grid');
  if (!grid) return;

  const presetIcons = {
    latch: '<svg width="24" height="24" viewBox="0 0 200 200" fill="none"><path d="M 50 35 L 25 35 Q 15 35 15 45 L 15 155 Q 15 165 25 165 L 50 165" fill="none" stroke="currentColor" stroke-width="12" stroke-linecap="round"/><path d="M 150 35 L 175 35 Q 185 35 185 45 L 185 155 Q 185 165 175 165 L 150 165" fill="none" stroke="currentColor" stroke-width="12" stroke-linecap="round"/><line x1="50" y1="75" x2="150" y2="75" stroke="currentColor" stroke-width="4"/><line x1="50" y1="125" x2="150" y2="125" stroke="currentColor" stroke-width="4"/><line x1="65" y1="115" x2="135" y2="85" stroke="currentColor" stroke-width="8" stroke-linecap="round"/><rect x="58" y="108" width="18" height="14" rx="3" fill="#e85d3b"/><rect x="130" y="80" width="14" height="10" rx="2" fill="#e85d3b"/></svg>',
    video: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
    lightbulb: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18h6M10 22h4M12 2v1M4.22 4.22l.707.707M1 12h1m17 0h1m-2.927-6.373l.707-.707M18 12a6 6 0 1 0-12 0c0 2.21 1.343 4.107 3.254 4.909A3.75 3.75 0 0 1 12 21a3.75 3.75 0 0 1 2.746-4.091C16.657 16.107 18 14.21 18 12Z"/></svg>',
    music: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>',
    cpu: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="14" x2="23" y2="14"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="14" x2="4" y2="14"/></svg>',
    globe: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>',
    zap: '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>',
  };

  grid.innerHTML = presets.map(preset => `
    <div class="preset-card" data-preset-id="${preset.id}">
      <div class="preset-card-icon">${presetIcons[preset.icon] || presetIcons.zap}</div>
      <div class="preset-card-title">${preset.name}</div>
      <div class="preset-card-desc">${preset.description}</div>
      <div class="preset-card-tags">
        ${preset.tags.slice(0, 3).map(tag => `<span class="preset-tag">${tag}</span>`).join('')}
      </div>
    </div>
  `).join('');

  // Add click handlers
  grid.querySelectorAll('.preset-card').forEach(card => {
    card.addEventListener('click', () => {
      const presetId = card.dataset.presetId;
      applyPreset(presetId);
      $('preset-modal')?.close();
    });
  });
}

async function applyPreset(presetId) {
  const preset = getPreset(presetId);
  if (!preset) {
    showNotification(`Preset not found: ${presetId}`, 'error');
    return;
  }

  showNotification(`Applying preset: ${preset.name}...`, 'info');

  // Stop existing servers
  for (const server of state.servers) {
    try {
      if (window.clasp) {
        await window.clasp.stopServer(server.id);
      }
    } catch (e) {
      console.warn('Error stopping server:', e);
    }
  }
  state.servers = [];

  // Apply preset servers
  for (const serverConfig of preset.servers) {
    const config = {
      ...serverConfig,
      id: Date.now().toString() + Math.random().toString(36).substring(2, 9),
      protocol: serverConfig.protocol || serverConfig.type, // Ensure protocol is set
      status: 'starting',
    };

    try {
      if (window.clasp) {
        const result = await window.clasp.startServer(config);
        config.id = result?.id || config.id;
        config.status = 'connected';
      } else {
        config.status = 'connected';
      }
      state.servers.push(config);
    } catch (err) {
      config.status = 'error';
      config.error = err.message;
      state.servers.push(config);
    }
  }

  // Apply preset bridges
  state.bridges = [];
  for (const bridgeConfig of preset.bridges) {
    const config = {
      ...bridgeConfig,
      id: Date.now().toString() + Math.random().toString(36).substring(2, 9),
      active: false,
    };

    try {
      if (window.clasp) {
        const result = await window.clasp.createBridge(config);
        config.id = result?.id || config.id;
        config.active = true;
      } else {
        config.active = true;
      }
      state.bridges.push(config);
    } catch (err) {
      state.bridges.push(config);
    }
  }

  // Apply preset mappings
  state.mappings = preset.mappings.map((m, i) => ({
    ...m,
    id: Date.now().toString() + i,
    enabled: true,
  }));

  // Save and render
  saveServersToStorage();
  saveBridgesToStorage();
  saveMappingsToStorage();
  renderServers();
  renderBridges();
  renderMappings();
  renderFlowDiagram();
  updateStatus();

  showNotification(`Preset "${preset.name}" applied successfully!`, 'success');
}

// ============================================
// Onboarding Wizard
// ============================================

function setupOnboarding() {
  const useCaseBtns = document.querySelectorAll('.use-case-btn');
  useCaseBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      useCaseBtns.forEach(b => b.classList.remove('selected'));
      btn.classList.add('selected');
      state.selectedUseCase = btn.dataset.useCase;
    });
  });

  $('onboarding-next')?.addEventListener('click', () => {
    if (state.onboardingStep === 2 && !state.selectedUseCase) {
      showNotification('Please select a use case', 'warning');
      return;
    }
    goToOnboardingStep(state.onboardingStep + 1);
  });

  $('onboarding-back')?.addEventListener('click', () => {
    goToOnboardingStep(state.onboardingStep - 1);
  });

  $('onboarding-skip')?.addEventListener('click', () => {
    finishOnboarding(false);
  });

  $('onboarding-finish')?.addEventListener('click', () => {
    finishOnboarding(true);
  });

  // Dot navigation
  document.querySelectorAll('.onboarding-dot').forEach(dot => {
    dot.addEventListener('click', () => {
      const step = parseInt(dot.dataset.step);
      if (step < state.onboardingStep) {
        goToOnboardingStep(step);
      }
    });
  });
}

function goToOnboardingStep(step) {
  state.onboardingStep = step;

  // Update step visibility
  document.querySelectorAll('.onboarding-step').forEach(s => {
    s.classList.toggle('active', parseInt(s.dataset.step) === step);
  });

  // Update dots
  document.querySelectorAll('.onboarding-dot').forEach(dot => {
    const dotStep = parseInt(dot.dataset.step);
    dot.classList.toggle('active', dotStep === step);
    dot.classList.toggle('completed', dotStep < step);
  });

  // Update buttons
  const backBtn = $('onboarding-back');
  const nextBtn = $('onboarding-next');
  const skipBtn = $('onboarding-skip');
  const finishBtn = $('onboarding-finish');

  if (step === 1) {
    backBtn?.classList.add('hidden');
    nextBtn?.classList.remove('hidden');
    skipBtn?.classList.remove('hidden');
    finishBtn?.classList.add('hidden');
  } else if (step === 2) {
    backBtn?.classList.remove('hidden');
    nextBtn?.classList.remove('hidden');
    skipBtn?.classList.add('hidden');
    finishBtn?.classList.add('hidden');
  } else if (step === 3) {
    backBtn?.classList.add('hidden');
    nextBtn?.classList.add('hidden');
    skipBtn?.classList.add('hidden');
    finishBtn?.classList.remove('hidden');

    // Apply preset based on use case
    if (state.selectedUseCase && state.selectedUseCase !== 'custom') {
      const presetMap = {
        'vj': 'vj-setup',
        'lighting': 'lighting-console',
        'music': 'midi-hub',
        'iot': 'sensor-network',
        'web': 'web-control',
      };
      const presetId = presetMap[state.selectedUseCase];
      if (presetId) {
        applyPreset(presetId);
        $('onboarding-summary').textContent = `We've configured CLASP Bridge with the "${getPreset(presetId)?.name}" preset.`;
      }
    } else {
      $('onboarding-summary').textContent = 'Start by adding servers and bridges to build your custom setup.';
    }
  }
}

async function checkFirstRun() {
  try {
    if (window.clasp) {
      const isFirst = await window.clasp.isFirstRun();
      if (isFirst) {
        $('onboarding-modal')?.showModal();
      }
    }
  } catch (e) {
    // First run check failed - continue without onboarding
  }
}

async function finishOnboarding() {
  $('onboarding-modal')?.close();

  try {
    if (window.clasp) {
      await window.clasp.setFirstRunComplete();
    }
  } catch (e) {
    // Could not persist first run state - non-critical
  }
}

// ============================================
// Config Import/Export
// ============================================

function setupConfigButtons() {
  $('import-btn')?.addEventListener('click', handleConfigImport);
  $('export-btn')?.addEventListener('click', handleConfigExport);
}

async function handleConfigExport() {
  try {
    if (window.clasp) {
      const result = await window.clasp.showSaveDialog({
        title: 'Export CLASP Configuration',
        defaultPath: 'clasp-config.json',
      });

      if (!result.canceled && result.filePath) {
        const config = exportConfig(state);
        const json = JSON.stringify(config, null, 2);
        await window.clasp.writeFile(result.filePath, json);
        showNotification('Configuration exported successfully!', 'success');
      }
    } else {
      // Fallback to browser download
      downloadConfig(state);
      showNotification('Configuration downloaded!', 'success');
    }
  } catch (e) {
    console.error('Export failed:', e);
    showNotification(`Export failed: ${e.message}`, 'error');
  }
}

async function handleConfigImport() {
  try {
    if (window.clasp) {
      const result = await window.clasp.showOpenDialog({
        title: 'Import CLASP Configuration',
      });

      if (!result.canceled && result.filePaths?.length > 0) {
        const fileResult = await window.clasp.readFile(result.filePaths[0]);
        if (fileResult.success) {
          const config = JSON.parse(fileResult.content);
          const validated = importConfig(config);
          await applyImportedConfig(validated);
          showNotification('Configuration imported successfully!', 'success');
        } else {
          showNotification(`Failed to read file: ${fileResult.error}`, 'error');
        }
      }
    } else {
      // Fallback to file input
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.json';
      input.onchange = async (e) => {
        const file = e.target.files[0];
        if (file) {
          try {
            const validated = await loadConfigFromFile(file);
            await applyImportedConfig(validated);
            showNotification('Configuration imported successfully!', 'success');
          } catch (err) {
            showNotification(`Import failed: ${err.message}`, 'error');
          }
        }
      };
      input.click();
    }
  } catch (e) {
    console.error('Import failed:', e);
    showNotification(`Import failed: ${e.message}`, 'error');
  }
}

async function applyImportedConfig(config) {
  // Stop existing servers
  for (const server of state.servers) {
    try {
      if (window.clasp) {
        await window.clasp.stopServer(server.id);
      }
    } catch (e) {
      console.warn('Error stopping server:', e);
    }
  }

  // Apply imported config
  state.servers = [];
  for (const serverConfig of config.servers) {
    try {
      if (window.clasp) {
        const result = await window.clasp.startServer(serverConfig);
        serverConfig.id = result?.id || serverConfig.id;
        serverConfig.status = 'connected';
      } else {
        serverConfig.status = 'connected';
      }
      state.servers.push(serverConfig);
    } catch (err) {
      serverConfig.status = 'error';
      serverConfig.error = err.message;
      state.servers.push(serverConfig);
    }
  }

  state.bridges = config.bridges;
  state.mappings = config.mappings;

  saveServersToStorage();
  saveBridgesToStorage();
  saveMappingsToStorage();
  renderServers();
  renderBridges();
  renderMappings();
  renderFlowDiagram();
  updateStatus();
}

// ============================================
// Flow Diagram
// ============================================

function setupFlowDiagram() {
  $('auto-layout-btn')?.addEventListener('click', () => {
    renderFlowDiagram();
  });

  // Re-render on tab change
  const flowTab = document.querySelector('[data-tab="flow"]');
  if (flowTab) {
    const observer = new MutationObserver(() => {
      if (document.querySelector('#panel-flow.active')) {
        renderFlowDiagram();
      }
    });
    observer.observe(document.querySelector('#panel-flow'), { attributes: true, attributeFilter: ['class'] });
  }

  // Re-render on window resize (debounced)
  let resizeTimeout;
  window.addEventListener('resize', () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      if (document.querySelector('#panel-flow.active')) {
        renderFlowDiagram();
      }
    }, 100);
  });
}

function renderFlowDiagram() {
  const nodesContainer = $('flow-nodes');
  const canvas = $('flow-canvas');
  if (!nodesContainer || !canvas) return;

  const container = nodesContainer.parentElement;
  const width = container.clientWidth;
  const height = container.clientHeight;

  // Don't render if container has no size (hidden tab)
  if (width < 100 || height < 100) return;

  // Resize canvas
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, width, height);

  // Check if we have any nodes (routers, protocol connections, or bridges)
  if (state.routers.length === 0 && state.servers.length === 0 && state.bridges.length === 0) {
    nodesContainer.innerHTML = `
      <div class="flow-empty">
        <div class="flow-empty-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 12h16M8 8l-4 4 4 4M16 8l4 4-4 4"/>
          </svg>
        </div>
        <div>No routers, connections, or bridges configured</div>
        <div style="font-size: 11px; opacity: 0.7; margin-top: 4px;">Add routers and protocol connections from the sidebar to see them here</div>
      </div>
    `;
    return;
  }

  // Responsive layout parameters
  const padding = Math.max(40, width * 0.05);
  const nodeWidth = Math.min(150, Math.max(100, width * 0.15));
  const nodeHeight = 60;
  const nodeGap = 20;

  // Layout: sources on left, CLASP hub in center, targets on right
  const leftX = padding;
  const centerX = width / 2 - nodeWidth / 2;
  const rightX = width - padding - nodeWidth;

  // Categorize by type (use 'type' field first, then 'protocol')
  const getServerType = (s) => s.type || s.protocol;
  // Protocol connections (sources and targets)
  const sourceServers = state.servers.filter(s => ['osc', 'midi', 'mqtt', 'websocket', 'http'].includes(getServerType(s)));
  const targetServers = state.servers.filter(s => ['artnet', 'dmx'].includes(getServerType(s)));
  // CLASP routers (hubs) - now in separate state.routers array
  const claspRouters = state.routers;

  // Calculate vertical centering
  const sourceCount = sourceServers.length;
  const targetCount = targetServers.length;
  const maxCount = Math.max(sourceCount, targetCount, 1);

  const totalSourceHeight = sourceCount * nodeHeight + (sourceCount - 1) * nodeGap;
  const totalTargetHeight = targetCount * nodeHeight + (targetCount - 1) * nodeGap;

  const sourceStartY = Math.max(padding, (height - totalSourceHeight) / 2);
  const targetStartY = Math.max(padding, (height - totalTargetHeight) / 2);
  const hubY = Math.max(padding, (height - nodeHeight) / 2);

  // Calculate positions
  const nodes = [];

  // Source nodes (left)
  sourceServers.forEach((server, i) => {
    nodes.push({
      id: server.id,
      type: 'source',
      x: leftX,
      y: sourceStartY + i * (nodeHeight + nodeGap),
      width: nodeWidth,
      server,
    });
  });

  // CLASP hub (center) - use routers from state.routers
  if (claspRouters.length > 0 || state.bridges.length > 0 || sourceServers.length > 0) {
    nodes.push({
      id: 'clasp-hub',
      type: 'hub',
      x: centerX,
      y: hubY,
      width: nodeWidth,
      server: claspRouters[0] || { name: 'CLASP Router', type: 'clasp', protocol: 'clasp', status: 'connected' },
    });
  }

  // Target nodes (right)
  targetServers.forEach((server, i) => {
    nodes.push({
      id: server.id,
      type: 'target',
      x: rightX,
      y: targetStartY + i * (nodeHeight + nodeGap),
      width: nodeWidth,
      server,
    });
  });

  // Render nodes
  nodesContainer.innerHTML = nodes.map(node => {
    const isHub = node.type === 'hub';
    const status = node.server.status === 'connected' || node.server.status === 'running' ? 'active' : '';
    const serverType = node.server.type || node.server.protocol;

    return `
      <div class="flow-node ${isHub ? 'flow-node-hub' : ''}" style="left: ${node.x}px; top: ${node.y}px; width: ${node.width}px;" data-node-id="${node.id}">
        <span class="flow-node-status ${status}"></span>
        <div class="flow-node-title">${protocolNames[serverType] || node.server.name || 'CLASP Hub'}</div>
        <div class="flow-node-detail">${node.server.address || ''}</div>
      </div>
    `;
  }).join('');

  // Draw connections
  ctx.strokeStyle = '#14b8a6';
  ctx.lineWidth = 2;
  ctx.setLineDash([5, 5]);

  const hubNode = nodes.find(n => n.type === 'hub');
  if (hubNode) {
    const hubCenterY = hubNode.y + nodeHeight / 2;
    const hubLeftX = hubNode.x;
    const hubRightX = hubNode.x + hubNode.width;

    // Draw lines from sources to hub
    nodes.filter(n => n.type === 'source').forEach(source => {
      const sourceRightX = source.x + source.width;
      const sourceCenterY = source.y + nodeHeight / 2;

      ctx.beginPath();
      ctx.moveTo(sourceRightX, sourceCenterY);
      ctx.lineTo(hubLeftX, hubCenterY);
      ctx.stroke();
    });

    // Draw lines from hub to targets
    nodes.filter(n => n.type === 'target').forEach(target => {
      const targetLeftX = target.x;
      const targetCenterY = target.y + nodeHeight / 2;

      ctx.beginPath();
      ctx.moveTo(hubRightX, hubCenterY);
      ctx.lineTo(targetLeftX, targetCenterY);
      ctx.stroke();
    });
  }
}

// ============================================
// Log Viewer
// ============================================

function setupLogViewer() {
  $('clear-logs-btn')?.addEventListener('click', () => {
    state.systemLogs = [];
    state.serverLogs.clear();
    renderLogs();
  });

  $('export-logs-btn')?.addEventListener('click', exportLogs);

  $('log-filter-level')?.addEventListener('change', renderLogs);
  $('log-filter-server')?.addEventListener('change', renderLogs);

  // Debounced search input
  let searchTimeout;
  $('log-search')?.addEventListener('input', () => {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(renderLogs, 150);
  });
}

function renderLogs() {
  const viewer = $('log-viewer');
  const statsEl = $('log-stats');
  if (!viewer) return;

  const levelFilter = $('log-filter-level')?.value || 'all';
  const serverFilter = $('log-filter-server')?.value || 'all';
  const searchQuery = ($('log-search')?.value || '').toLowerCase().trim();

  // Combine system logs and server logs
  let allLogs = [...state.systemLogs];

  state.serverLogs.forEach((logs, serverId) => {
    logs.forEach(log => {
      allLogs.push({
        ...log,
        source: serverId,
      });
    });
  });

  // Sort by timestamp descending
  allLogs.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));

  // Count by level for stats (before filtering)
  const totalCounts = {
    error: allLogs.filter(l => l.level === 'error').length,
    warning: allLogs.filter(l => l.level === 'warning').length,
    info: allLogs.filter(l => l.level === 'info').length,
    debug: allLogs.filter(l => l.level === 'debug').length,
  };

  // Apply level filter
  if (levelFilter !== 'all') {
    const levels = {
      'error': ['error'],
      'warning': ['error', 'warning'],
      'info': ['error', 'warning', 'info'],
      'debug': ['error', 'warning', 'info', 'debug'],
    };
    allLogs = allLogs.filter(log => levels[levelFilter]?.includes(log.level));
  }

  // Apply server filter
  if (serverFilter !== 'all') {
    allLogs = allLogs.filter(log => log.source === serverFilter);
  }

  // Apply search filter
  if (searchQuery) {
    allLogs = allLogs.filter(log => 
      log.message?.toLowerCase().includes(searchQuery) ||
      log.source?.toLowerCase().includes(searchQuery) ||
      log.level?.toLowerCase().includes(searchQuery)
    );
  }

  // Update stats
  if (statsEl) {
    statsEl.innerHTML = `
      <span class="log-stat log-stat-error" title="Errors">${totalCounts.error} errors</span>
      <span class="log-stat log-stat-warning" title="Warnings">${totalCounts.warning} warnings</span>
      <span class="log-stat log-stat-info" title="Info">${totalCounts.info} info</span>
      ${searchQuery ? `<span class="log-stat log-stat-search">${allLogs.length} matches</span>` : ''}
    `;
  }

  if (allLogs.length === 0) {
    viewer.innerHTML = searchQuery 
      ? '<div class="log-empty">No logs match your search</div>'
      : '<div class="log-empty">No logs to display</div>';
    return;
  }

  // Render logs with search highlighting
  viewer.innerHTML = allLogs.slice(0, 500).map(log => {
    const time = new Date(log.timestamp).toLocaleTimeString();
    let message = escapeHtml(log.message);
    
    // Highlight search matches
    if (searchQuery) {
      const regex = new RegExp(`(${escapeRegex(searchQuery)})`, 'gi');
      message = message.replace(regex, '<mark>$1</mark>');
    }
    
    return `
      <div class="log-entry log-entry-${log.level}">
        <span class="log-timestamp">${time}</span>
        <span class="log-level log-level-${log.level}">${log.level.toUpperCase()}</span>
        <span class="log-source">${log.source || 'System'}</span>
        <span class="log-message">${message}</span>
      </div>
    `;
  }).join('');

  // Update server filter dropdown
  const serverSelect = $('log-filter-server');
  if (serverSelect) {
    const currentVal = serverSelect.value;
    const servers = [...new Set(state.systemLogs.map(l => l.source).filter(Boolean))];
    state.servers.forEach(s => {
      if (!servers.includes(s.id)) servers.push(s.id);
    });

    serverSelect.innerHTML = '<option value="all">All Sources</option>' +
      servers.map(s => {
        const server = state.servers.find(srv => srv.id === s);
        const name = server?.name || s;
        return `<option value="${s}">${name}</option>`;
      }).join('');

    serverSelect.value = currentVal;
  }
}

function escapeRegex(string) {
  return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function exportLogs() {
  const logs = state.systemLogs.map(log => {
    return `[${new Date(log.timestamp).toISOString()}] [${log.level.toUpperCase()}] [${log.source || 'System'}] ${log.message}`;
  }).join('\n');

  const blob = new Blob([logs], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `clasp-logs-${new Date().toISOString().split('T')[0]}.txt`;
  link.click();
  URL.revokeObjectURL(url);
  showNotification('Logs exported!', 'success');
}

// ============================================
// Enhanced Signal Monitor with Sparklines
// ============================================

function addSignalToHistory(signal) {
  const address = signal.address || signal.bridgeId || 'unknown';
  const value = typeof signal.value === 'number' ? signal.value :
                signal.velocity !== undefined ? signal.velocity / 127 : 0;

  if (!state.signalHistory.has(address)) {
    state.signalHistory.set(address, {
      values: [],
      updateCount: 0,
      lastUpdate: Date.now(),
    });
  }

  const history = state.signalHistory.get(address);
  history.values.push(value);
  history.updateCount++;
  history.lastUpdate = Date.now();

  // Keep last 50 values for sparkline
  if (history.values.length > 50) {
    history.values.shift();
  }

  // Periodically clean up stale entries (every 100 updates, remove entries older than 5 minutes)
  if (history.updateCount % 100 === 0) {
    cleanupStaleSignalHistory();
  }
}

function cleanupStaleSignalHistory() {
  const staleThreshold = 5 * 60 * 1000; // 5 minutes
  const now = Date.now();
  const maxEntries = 500; // Maximum unique addresses to track

  // Remove entries older than threshold
  for (const [address, history] of state.signalHistory) {
    if (now - history.lastUpdate > staleThreshold) {
      state.signalHistory.delete(address);
    }
  }

  // If still too many entries, remove oldest ones
  if (state.signalHistory.size > maxEntries) {
    const entries = [...state.signalHistory.entries()];
    entries.sort((a, b) => a[1].lastUpdate - b[1].lastUpdate);
    const toRemove = entries.slice(0, entries.length - maxEntries);
    for (const [address] of toRemove) {
      state.signalHistory.delete(address);
    }
  }
}

// ============================================
// Test Panel & Diagnostics
// ============================================

function setupTestPanel() {
  // Run diagnostics button
  $('run-diagnostics-btn')?.addEventListener('click', runDiagnostics);

  // Send test signal button
  $('send-test-signal-btn')?.addEventListener('click', sendTestSignal);

  // Continuous test button
  $('send-continuous-btn')?.addEventListener('click', toggleContinuousTest);

  // Update test target dropdown when tab is shown
  const testTab = document.querySelector('[data-tab="test"]');
  testTab?.addEventListener('click', () => {
    setTimeout(updateTestTargetDropdown, 100);
  });
}

function setupServerStatsUpdates() {
  if (!window.clasp) return;

  // Listen for periodic stats updates from backend
  window.clasp.onServerStatsUpdate?.((stats) => {
    for (const stat of stats) {
      state.serverStats.set(stat.id, stat);
    }
    // Update UI if on test panel
    if (state.activeTab === 'test') {
      renderServerHealth();
    }
    // Update server list with live stats
    renderServerStats();
  });

  // Listen for bridge service ready status
  window.clasp.onBridgeReady?.((ready) => {
    state.bridgeServiceReady = ready;
    console.log(`Bridge service ready: ${ready}`);
    updateBridgeServiceStatus();
    if (!ready) {
      showNotification('Bridge service disconnected', 'error');
    } else {
      showNotification('Bridge service connected', 'success');
    }
  });

  // Listen for bridge-to-router connection status
  window.clasp.onBridgeRouterStatus?.((status) => {
    const server = state.servers.find(s => s.id === status.bridgeId);
    if (server) {
      server.routerConnected = status.connected;
      server.routerError = status.error || null;
      server.connectedRouterId = status.routerId || null;
      renderServers();
    }
  });

  // Check initial bridge status
  checkBridgeServiceStatus();
}

async function checkBridgeServiceStatus() {
  try {
    const status = await window.clasp.getBridgeStatus();
    state.bridgeServiceReady = status.ready;
    console.log('Initial bridge service status:', status);
    updateBridgeServiceStatus();
  } catch (e) {
    console.error('Failed to check bridge status:', e);
    state.bridgeServiceReady = false;
    updateBridgeServiceStatus();
  }
}

function updateBridgeServiceStatus() {
  // Update status bar or indicator
  const statusEl = $('status-text');
  if (statusEl && !state.bridgeServiceReady) {
    // Show warning in status
    statusEl.classList.add('warning');
  } else if (statusEl) {
    statusEl.classList.remove('warning');
  }

  // Update server cards that might show "Bridge service not ready"
  renderServers();
}

function updateTestTargetDropdown() {
  const select = $('test-target-server');
  if (!select) return;

  const currentValue = select.value;
  select.innerHTML = '<option value="">Select a protocol connection...</option>';

  for (const server of state.servers) {
    const option = document.createElement('option');
    option.value = server.id;
    option.textContent = `${server.name} (${server.protocol || server.type})`;
    select.appendChild(option);
  }

  if ([...select.options].some(o => o.value === currentValue)) {
    select.value = currentValue;
  }
}

async function sendTestSignal() {
  const targetEl = $('test-target-server');
  const addressEl = $('test-signal-address');
  const valueTypeEl = $('test-value-type');
  const valueEl = $('test-signal-value');
  const resultEl = $('test-signal-result');

  const target = targetEl?.value;
  const signalAddress = addressEl?.value || '/test/signal';
  const valueType = valueTypeEl?.value || 'float';
  let rawValue = valueEl?.value || '0.5';

  if (!target) {
    if (resultEl) {
      resultEl.textContent = 'Please select a protocol connection';
      resultEl.className = 'form-hint error';
    }
    return;
  }

  // Parse value based on type
  let value;
  switch (valueType) {
    case 'float':
      value = parseFloat(rawValue) || 0;
      break;
    case 'int':
      value = parseInt(rawValue) || 0;
      break;
    case 'bool':
      value = rawValue === 'true' || rawValue === '1';
      break;
    case 'string':
      value = rawValue;
      break;
    default:
      value = rawValue;
  }

  // Find target config from servers (protocol connections)
  let protocol, address;
    const server = state.servers.find(s => s.id === target);
    if (server) {
      protocol = server.protocol || server.type;
      address = server.address;
  }

  if (!protocol || !address) {
    if (resultEl) {
      resultEl.textContent = 'Could not determine target address';
      resultEl.className = 'form-hint error';
    }
    return;
  }

  try {
    if (resultEl) {
      resultEl.textContent = 'Sending...';
      resultEl.className = 'form-hint testing';
    }

    const result = await window.clasp.sendTestSignal({
      protocol,
      address,
      signalAddress,
      value,
    });

    if (result.success) {
      if (resultEl) {
        resultEl.textContent = `Sent ${signalAddress} = ${value} to ${address}`;
        resultEl.className = 'form-hint success';
      }
    } else {
      if (resultEl) {
        resultEl.textContent = result.error || 'Failed to send signal';
        resultEl.className = 'form-hint error';
      }
    }
  } catch (e) {
    if (resultEl) {
      resultEl.textContent = e.message;
      resultEl.className = 'form-hint error';
    }
  }
}

function toggleContinuousTest() {
  const btn = $('send-continuous-btn');
  const resultEl = $('test-signal-result');

  if (state.continuousTestInterval) {
    // Stop
    clearInterval(state.continuousTestInterval);
    state.continuousTestInterval = null;
    if (btn) {
      btn.innerHTML = `
        <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
        START CONTINUOUS
      `;
      btn.classList.remove('btn-primary');
      btn.classList.add('btn-secondary');
    }
    if (resultEl) {
      resultEl.textContent = 'Stopped continuous test';
      resultEl.className = 'form-hint';
    }
  } else {
    // Start
    let counter = 0;
    state.continuousTestInterval = setInterval(() => {
      const valueEl = $('test-signal-value');
      // Oscillate value for demo
      const phase = (counter % 100) / 100;
      const value = Math.sin(phase * Math.PI * 2) * 0.5 + 0.5;
      if (valueEl) valueEl.value = value.toFixed(3);
      sendTestSignal();
      counter++;
    }, 100); // 10 Hz

    if (btn) {
      btn.innerHTML = `
        <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
        STOP
      `;
      btn.classList.remove('btn-secondary');
      btn.classList.add('btn-primary');
    }
    if (resultEl) {
      resultEl.textContent = 'Sending continuous test signals (10 Hz)...';
      resultEl.className = 'form-hint testing';
    }
  }
}

async function runDiagnostics() {
  const outputEl = $('diagnostics-output');
  if (!outputEl) {
    console.error('Diagnostics output element not found');
    return;
  }

  outputEl.innerHTML = '<div class="empty-state-small"><span class="empty-state-text">Running diagnostics...</span></div>';

  try {
    if (!window.clasp || !window.clasp.runDiagnostics) {
      throw new Error('CLASP API not available');
    }
    const diagnostics = await window.clasp.runDiagnostics();
    
    if (!diagnostics) {
      throw new Error('Diagnostics returned no data');
    }

    outputEl.innerHTML = `
      <div class="diagnostics-section">
        <div class="diagnostics-section-title">Bridge Service</div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Status</span>
          <span class="diagnostics-value ${diagnostics.bridgeService.running ? 'ok' : 'error'}">
            ${diagnostics.bridgeService.running ? 'Running' : 'Not Running'}
          </span>
        </div>
        ${diagnostics.bridgeService.pid ? `
          <div class="diagnostics-row">
            <span class="diagnostics-label">Process ID</span>
            <span class="diagnostics-value">${diagnostics.bridgeService.pid}</span>
          </div>
        ` : ''}
      </div>

      <div class="diagnostics-section">
        <div class="diagnostics-section-title">System</div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Platform</span>
          <span class="diagnostics-value">${diagnostics.system.platform}</span>
        </div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Node.js</span>
          <span class="diagnostics-value">${diagnostics.system.nodeVersion}</span>
        </div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Electron</span>
          <span class="diagnostics-value">${diagnostics.system.electronVersion}</span>
        </div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Uptime</span>
          <span class="diagnostics-value">${Math.floor(diagnostics.system.uptime / 60)}m ${Math.floor(diagnostics.system.uptime % 60)}s</span>
        </div>
        <div class="diagnostics-row">
          <span class="diagnostics-label">Memory (Heap)</span>
          <span class="diagnostics-value">${(diagnostics.system.memoryUsage.heapUsed / 1024 / 1024).toFixed(1)} MB / ${(diagnostics.system.memoryUsage.heapTotal / 1024 / 1024).toFixed(1)} MB</span>
        </div>
      </div>

      <div class="diagnostics-section">
        <div class="diagnostics-section-title">Servers (${diagnostics.servers.length})</div>
        ${diagnostics.servers.length === 0 ? `
          <div class="diagnostics-row">
            <span class="diagnostics-label">No servers running</span>
          </div>
        ` : diagnostics.servers.map(server => `
          <div class="diagnostics-row">
            <span class="diagnostics-label">${server.name || server.type}</span>
            <span class="diagnostics-value ${server.status === 'running' ? 'ok' : 'error'}">
              ${server.status} | ${server.messagesIn} in / ${server.messagesOut} out | ${server.errors} errors
            </span>
          </div>
        `).join('')}
      </div>
    `;
  } catch (e) {
    console.error('Diagnostics error:', e);
    outputEl.innerHTML = `<div class="diagnostics-section">
      <div class="diagnostics-section-title">Error</div>
      <div class="diagnostics-row">
        <span class="diagnostics-label">Error running diagnostics:</span>
        <span class="diagnostics-value error">${e.message || String(e)}</span>
      </div>
      ${e.stack ? `<div class="diagnostics-row">
        <span class="diagnostics-label">Stack:</span>
        <span class="diagnostics-value" style="font-size: 10px; font-family: monospace;">${e.stack}</span>
      </div>` : ''}
    </div>`;
  }
}

async function renderServerHealth() {
  const container = $('server-health');
  if (!container) return;

  // Update test target dropdown when rendering health
  updateTestTargetDropdown();

  if (state.servers.length === 0) {
    container.innerHTML = `
      <div class="empty-state-small">
        <span class="empty-state-text">No protocol connections running</span>
      </div>
    `;
    return;
  }

  // Get health for each server
  const healthCards = [];
  for (const server of state.servers) {
    const stats = state.serverStats.get(server.id) || {};
    const uptime = stats.uptime || 0;
    const uptimeStr = formatUptimeClient(uptime);

    // Determine health status
    let healthClass = 'healthy';
    let healthIcon = '✓';
    if (server.status === 'error') {
      healthClass = 'unhealthy';
      healthIcon = '✗';
    } else if (stats.errors > 0) {
      healthClass = 'warning';
      healthIcon = '!';
    }

    healthCards.push(`
      <div class="server-health-card" data-id="${server.id}">
        <div class="server-health-status ${healthClass}">${healthIcon}</div>
        <div class="server-health-info">
          <span class="server-health-name">${server.name}</span>
          <div class="server-health-stats">
            <span class="server-health-stat">
              <strong>${stats.messagesIn || 0}</strong> in
            </span>
            <span class="server-health-stat">
              <strong>${stats.messagesOut || 0}</strong> out
            </span>
            <span class="server-health-stat">
              <strong>${stats.errors || 0}</strong> errors
            </span>
            <span class="server-health-stat">
              Uptime: <strong>${uptimeStr}</strong>
            </span>
          </div>
        </div>
        <div class="server-health-actions">
          <button class="btn btn-sm btn-secondary" data-action="health-check" data-id="${server.id}">Check</button>
        </div>
      </div>
    `);
  }

  container.innerHTML = healthCards.join('');

  // Add click handlers for health check buttons
  container.querySelectorAll('[data-action="health-check"]').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const id = e.target.dataset.id;
      btn.textContent = '...';
      try {
        const result = await window.clasp.healthCheck(id);
        btn.textContent = result.healthy ? 'OK' : 'FAIL';
        setTimeout(() => { btn.textContent = 'Check'; }, 2000);
      } catch (err) {
        btn.textContent = 'ERR';
        setTimeout(() => { btn.textContent = 'Check'; }, 2000);
      }
    });
  });
}

function renderServerStats() {
  // Update the server list in sidebar with live stats
  const list = $('server-list');
  if (!list) return;

  // Don't re-render if no servers - let renderServers handle empty state
  if (state.servers.length === 0) return;

  // Just update the stats inline without full re-render
  for (const [id, stats] of state.serverStats) {
    const item = list.querySelector(`[data-id="${id}"]`);
    if (item) {
      let statsRow = item.querySelector('.server-stats-row');
      if (!statsRow) {
        // Add stats row if not present
        item.classList.add('with-stats');
        statsRow = document.createElement('div');
        statsRow.className = 'server-stats-row';
        item.appendChild(statsRow);
      }
      statsRow.innerHTML = `
        <span class="server-stat">↓ <span class="server-stat-value">${stats.messagesIn || 0}</span></span>
        <span class="server-stat">↑ <span class="server-stat-value">${stats.messagesOut || 0}</span></span>
        <span class="server-stat">⚠ <span class="server-stat-value">${stats.errors || 0}</span></span>
      `;
    }
  }
}

function formatUptimeClient(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ${hours % 24}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

// ============================================
// Global exports (for inline onclick handlers)
// ============================================
window.deleteToken = deleteToken;
window.copyToken = copyToken;

// ============================================
// Initialize
// ============================================

// Prevent re-initialization on HMR (dev only)
// Use window flag since module-level vars reset on HMR
if (!window.__CLASP_INITIALIZED__) {
  document.addEventListener('DOMContentLoaded', () => {
    if (!window.__CLASP_INITIALIZED__) {
      window.__CLASP_INITIALIZED__ = true;
      init();
    }
  });

  // Also handle case where DOMContentLoaded already fired
  if (document.readyState !== 'loading' && !window.__CLASP_INITIALIZED__) {
    window.__CLASP_INITIALIZED__ = true;
    init();
  }
}

// Vite HMR - just accept updates, don't re-init
if (import.meta.hot) {
  import.meta.hot.accept();
}
