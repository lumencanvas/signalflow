const { contextBridge, ipcRenderer } = require('electron');

// Expose CLASP API to the renderer process
const api = {
  // Device/Server management
  getDevices: () => ipcRenderer.invoke('get-devices'),
  scanNetwork: () => ipcRenderer.invoke('scan-network'),
  addServer: (address) => ipcRenderer.invoke('add-server', address),
  startServer: (config) => ipcRenderer.invoke('start-server', config),
  stopServer: (id) => ipcRenderer.invoke('stop-server', id),

  // Bridge management
  getBridges: () => ipcRenderer.invoke('get-bridges'),
  createBridge: (config) => ipcRenderer.invoke('create-bridge', config),
  deleteBridge: (id) => ipcRenderer.invoke('delete-bridge', id),

  // Server logs and diagnostics
  getServerLogs: (id) => ipcRenderer.invoke('get-server-logs', id),
  testConnection: (address) => ipcRenderer.invoke('test-connection', address),

  // Hardware discovery
  listSerialPorts: () => ipcRenderer.invoke('list-serial-ports'),
  listMidiPorts: () => ipcRenderer.invoke('list-midi-ports'),
  listNetworkInterfaces: () => ipcRenderer.invoke('list-network-interfaces'),
  testSerialPort: (portPath) => ipcRenderer.invoke('test-serial-port', portPath),
  testPortAvailable: (host, port) => ipcRenderer.invoke('test-port-available', { host, port }),

  // Server stats & diagnostics
  getServerStats: (id) => ipcRenderer.invoke('get-server-stats', id),
  getAllServerStats: () => ipcRenderer.invoke('get-all-server-stats'),
  healthCheck: (id) => ipcRenderer.invoke('health-check', id),
  runDiagnostics: () => ipcRenderer.invoke('run-diagnostics'),
  getBridgeStatus: () => ipcRenderer.invoke('get-bridge-status'),

  // Test signal generator
  sendTestSignal: (config) => ipcRenderer.invoke('send-test-signal', config),
  sendTestSignalBatch: (signals) => ipcRenderer.invoke('send-test-signal-batch', { signals }),

  // Signal routing
  sendSignal: (bridgeId, address, value) =>
    ipcRenderer.invoke('send-signal', { bridgeId, address, value }),

  // Learn mode
  startLearnMode: (target) => ipcRenderer.invoke('start-learn-mode', target),
  stopLearnMode: () => ipcRenderer.invoke('stop-learn-mode'),

  // Configuration
  getAppVersion: () => ipcRenderer.invoke('get-app-version'),
  isFirstRun: () => ipcRenderer.invoke('is-first-run'),
  setFirstRunComplete: () => ipcRenderer.invoke('set-first-run-complete'),

  // File dialogs (for config import/export)
  showSaveDialog: (options) => ipcRenderer.invoke('show-save-dialog', options),
  showOpenDialog: (options) => ipcRenderer.invoke('show-open-dialog', options),
  writeFile: (path, content) => ipcRenderer.invoke('write-file', { path, content }),
  readFile: (path) => ipcRenderer.invoke('read-file', path),

  // Events
  onDeviceFound: (callback) => {
    ipcRenderer.on('device-found', (event, device) => callback(device));
    return () => ipcRenderer.removeAllListeners('device-found');
  },
  onDeviceUpdated: (callback) => {
    ipcRenderer.on('device-updated', (event, device) => callback(device));
    return () => ipcRenderer.removeAllListeners('device-updated');
  },
  onDeviceLost: (callback) => {
    ipcRenderer.on('device-lost', (event, deviceId) => callback(deviceId));
    return () => ipcRenderer.removeAllListeners('device-lost');
  },
  onSignal: (callback) => {
    ipcRenderer.on('signal', (event, signal) => callback(signal));
    return () => ipcRenderer.removeAllListeners('signal');
  },
  onScanStarted: (callback) => {
    ipcRenderer.on('scan-started', () => callback());
    return () => ipcRenderer.removeAllListeners('scan-started');
  },
  onScanComplete: (callback) => {
    ipcRenderer.on('scan-complete', () => callback());
    return () => ipcRenderer.removeAllListeners('scan-complete');
  },
  onServerStatus: (callback) => {
    ipcRenderer.on('server-status', (event, status) => callback(status));
    return () => ipcRenderer.removeAllListeners('server-status');
  },
  onServerLog: (callback) => {
    ipcRenderer.on('server-log', (event, data) => callback(data));
    return () => ipcRenderer.removeAllListeners('server-log');
  },
  onBridgeEvent: (callback) => {
    ipcRenderer.on('bridge-event', (event, data) => callback(data));
    return () => ipcRenderer.removeAllListeners('bridge-event');
  },
  onLearnedSignal: (callback) => {
    ipcRenderer.on('learned-signal', (event, signal) => callback(signal));
    return () => ipcRenderer.removeAllListeners('learned-signal');
  },
  onServerStatsUpdate: (callback) => {
    ipcRenderer.on('server-stats-update', (event, stats) => callback(stats));
    return () => ipcRenderer.removeAllListeners('server-stats-update');
  },
  onBridgeReady: (callback) => {
    ipcRenderer.on('bridge-ready', (event, ready) => callback(ready));
    return () => ipcRenderer.removeAllListeners('bridge-ready');
  },
  onBridgeRouterStatus: (callback) => {
    ipcRenderer.on('bridge-router-status', (event, status) => callback(status));
    return () => ipcRenderer.removeAllListeners('bridge-router-status');
  },
};

contextBridge.exposeInMainWorld('clasp', api);
