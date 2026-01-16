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

  // Signal routing
  sendSignal: (bridgeId, address, value) =>
    ipcRenderer.invoke('send-signal', { bridgeId, address, value }),

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
};

contextBridge.exposeInMainWorld('clasp', api);
