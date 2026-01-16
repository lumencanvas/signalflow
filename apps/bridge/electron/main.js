const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const readline = require('readline');
const WebSocket = require('ws');
const os = require('os');

let mainWindow;
let bridgeService = null;
let bridgeReady = false;

// Track running server processes
const runningServers = new Map(); // id -> { process, config, status, logs }
const MAX_LOG_LINES = 500;

// Path to binaries
const getBinaryPath = (name) => {
  // In development, use the cargo build output
  const devPath = path.join(__dirname, '..', '..', '..', 'target', 'release', name);
  // In production, binaries are bundled
  const prodPath = path.join(process.resourcesPath || '', 'bin', name);

  // Detect dev mode - if app is not packaged (running from source)
  const isDev = !app.isPackaged;

  if (isDev) {
    return devPath;
  }
  return prodPath;
};

// Start the bridge service (for protocol bridges)
function startBridgeService() {
  const servicePath = getBinaryPath('clasp-service');
  console.log('Starting bridge service:', servicePath);

  try {
    bridgeService = spawn(servicePath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    const rl = readline.createInterface({
      input: bridgeService.stdout,
      crlfDelay: Infinity,
    });

    rl.on('line', (line) => {
      try {
        const message = JSON.parse(line);
        handleBridgeMessage(message);
      } catch (e) {
        console.log('[bridge-service stdout]', line);
      }
    });

    bridgeService.stderr.on('data', (data) => {
      console.log('[bridge-service]', data.toString().trim());
    });

    bridgeService.on('close', (code) => {
      console.log('Bridge service exited with code:', code);
      bridgeService = null;
      bridgeReady = false;
    });

    bridgeService.on('error', (err) => {
      console.error('Bridge service error:', err);
      bridgeService = null;
      bridgeReady = false;
    });

  } catch (err) {
    console.error('Failed to start bridge service:', err);
  }
}

// Stop the bridge service
function stopBridgeService() {
  if (bridgeService) {
    sendToBridge({ type: 'shutdown' });
    setTimeout(() => {
      if (bridgeService) {
        bridgeService.kill();
        bridgeService = null;
      }
    }, 1000);
  }
}

// Send a message to the bridge service
function sendToBridge(message) {
  if (bridgeService && bridgeService.stdin) {
    const json = JSON.stringify(message);
    bridgeService.stdin.write(json + '\n');
  }
}

// Handle messages from the bridge service
function handleBridgeMessage(message) {
  switch (message.type) {
    case 'ready':
      console.log('Bridge service ready');
      bridgeReady = true;
      break;

    case 'signal':
      // Forward signal to renderer
      if (mainWindow) {
        mainWindow.webContents.send('signal', {
          bridgeId: message.bridge_id,
          address: message.address,
          value: message.value,
        });
      }
      break;

    case 'bridge_event':
      // Forward bridge event to renderer
      if (mainWindow) {
        mainWindow.webContents.send('bridge-event', {
          bridgeId: message.bridge_id,
          event: message.event,
          data: message.data,
        });
      }
      break;
  }
}

// Start a CLASP server (spawns clasp-router)
async function startClaspServer(config) {
  const routerPath = getBinaryPath('clasp-router');
  const [host, port] = (config.address || 'localhost:7330').split(':');

  console.log(`Starting CLASP router on ${host}:${port}`);

  const args = [
    '--listen', `${host === 'localhost' ? '0.0.0.0' : host}:${port}`,
    '--name', config.name || 'CLASP Bridge Server',
  ];

  if (config.announce !== false) {
    args.push('--announce');
  }

  return new Promise((resolve, reject) => {
    try {
      const proc = spawn(routerPath, args, {
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      const serverState = {
        process: proc,
        config,
        status: 'starting',
        logs: [],
        port: parseInt(port),
      };

      const addLog = (message, type = 'info') => {
        serverState.logs.push({
          timestamp: Date.now(),
          message,
          type,
        });
        if (serverState.logs.length > MAX_LOG_LINES) {
          serverState.logs.shift();
        }
        // Forward log to renderer
        mainWindow?.webContents.send('server-log', {
          serverId: config.id,
          log: { timestamp: Date.now(), message, type },
        });
      };

      proc.stdout.on('data', (data) => {
        const lines = data.toString().trim().split('\n');
        for (const line of lines) {
          addLog(line, 'stdout');
          // Check for "ready" or "listening" messages
          if (line.includes('Listening on') || line.includes('Router ready') || line.includes('accepting connections')) {
            serverState.status = 'running';
            mainWindow?.webContents.send('server-status', {
              id: config.id,
              status: 'running',
            });
          }
        }
      });

      proc.stderr.on('data', (data) => {
        const lines = data.toString().trim().split('\n');
        for (const line of lines) {
          addLog(line, 'stderr');
          // tracing logs go to stderr - check for success messages
          if (line.includes('Listening on') || line.includes('Router ready') || line.includes('accepting connections')) {
            serverState.status = 'running';
            mainWindow?.webContents.send('server-status', {
              id: config.id,
              status: 'running',
            });
          }
        }
      });

      proc.on('close', (code) => {
        addLog(`Process exited with code ${code}`, code === 0 ? 'info' : 'error');
        serverState.status = code === 0 ? 'stopped' : 'error';
        mainWindow?.webContents.send('server-status', {
          id: config.id,
          status: serverState.status,
          exitCode: code,
        });
        runningServers.delete(config.id);
      });

      proc.on('error', (err) => {
        addLog(`Process error: ${err.message}`, 'error');
        serverState.status = 'error';
        serverState.error = err.message;
        mainWindow?.webContents.send('server-status', {
          id: config.id,
          status: 'error',
          error: err.message,
        });
        reject(new Error(err.message));
      });

      runningServers.set(config.id, serverState);

      // Wait briefly to check if process started successfully
      setTimeout(() => {
        if (serverState.status === 'starting' && proc.exitCode === null) {
          serverState.status = 'running';
          mainWindow?.webContents.send('server-status', {
            id: config.id,
            status: 'running',
          });
        }
        resolve({ id: config.id, status: serverState.status });
      }, 500);

    } catch (err) {
      reject(err);
    }
  });
}

// Start an OSC server (via clasp-service bridge)
async function startOscServer(config) {
  const addr = `${config.bind || '0.0.0.0'}:${config.port || 9000}`;

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  // Create an OSC bridge that listens for incoming OSC
  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'osc',
    source_addr: addr,
    target: 'clasp',
    target_addr: 'internal',
  });

  const serverState = {
    process: null, // managed by bridge service
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `OSC listening on ${addr}`, type: 'info' }],
    port: config.port || 9000,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Start an MQTT client
async function startMqttServer(config) {
  const addr = `${config.host || 'localhost'}:${config.port || 1883}`;

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'mqtt',
    source_addr: addr,
    target: 'clasp',
    target_addr: 'internal',
    config: {
      topics: config.topics || ['#'],
    },
  });

  const serverState = {
    process: null,
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `MQTT connecting to ${addr}`, type: 'info' }],
    port: config.port || 1883,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Start a WebSocket server
async function startWebSocketServer(config) {
  const addr = config.address || '0.0.0.0:8080';

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'websocket',
    source_addr: addr,
    target: 'clasp',
    target_addr: 'internal',
    config: {
      mode: config.mode || 'server',
    },
  });

  const serverState = {
    process: null,
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `WebSocket ${config.mode || 'server'} on ${addr}`, type: 'info' }],
    port: parseInt(addr.split(':')[1]) || 8080,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Start an HTTP server
async function startHttpServer(config) {
  const addr = config.bind || '0.0.0.0:3000';

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'http',
    source_addr: addr,
    target: 'clasp',
    target_addr: 'internal',
    config: {
      base_path: config.basePath || '/api',
      cors: config.cors !== false,
    },
  });

  const serverState = {
    process: null,
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `HTTP API on ${addr}${config.basePath || '/api'}`, type: 'info' }],
    port: parseInt(addr.split(':')[1]) || 3000,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Start an Art-Net server
async function startArtNetServer(config) {
  const addr = config.bind || '0.0.0.0:6454';

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'artnet',
    source_addr: addr,
    target: 'clasp',
    target_addr: 'internal',
    config: {
      subnet: config.subnet || 0,
      universe: config.universe || 0,
    },
  });

  const serverState = {
    process: null,
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `Art-Net on ${addr} (${config.subnet}:${config.universe})`, type: 'info' }],
    port: 6454,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Start a DMX interface
async function startDmxServer(config) {
  const serialPort = config.serialPort || '/dev/ttyUSB0';

  if (!bridgeReady) {
    throw new Error('Bridge service not ready');
  }

  sendToBridge({
    type: 'create_bridge',
    id: config.id,
    source: 'dmx',
    source_addr: serialPort,
    target: 'clasp',
    target_addr: 'internal',
    config: {
      universe: config.universe || 0,
    },
  });

  const serverState = {
    process: null,
    config,
    status: 'running',
    logs: [{ timestamp: Date.now(), message: `DMX on ${serialPort} (U${config.universe || 0})`, type: 'info' }],
    port: null,
  };

  runningServers.set(config.id, serverState);

  return { id: config.id, status: 'running' };
}

// Stop a server by ID
async function stopServer(id) {
  const server = runningServers.get(id);
  if (!server) {
    return false;
  }

  if (server.process) {
    // For process-based servers (CLASP router)
    server.process.kill('SIGTERM');
    // Give it a moment to shut down gracefully
    await new Promise(resolve => setTimeout(resolve, 500));
    if (server.process && server.process.exitCode === null) {
      server.process.kill('SIGKILL');
    }
  } else {
    // For bridge-based servers
    if (bridgeReady) {
      sendToBridge({
        type: 'delete_bridge',
        id: id,
      });
    }
    runningServers.delete(id);
  }

  return true;
}

// Stop all running servers
async function stopAllServers() {
  const ids = Array.from(runningServers.keys());
  for (const id of ids) {
    await stopServer(id);
  }
}

// Create the main window
function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 900,
    minWidth: 900,
    minHeight: 600,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js'),
    },
    titleBarStyle: 'hiddenInset',
    backgroundColor: '#f5f5f4',
    show: false,
  });

  // Detect dev mode - if app is not packaged (running from source)
  const isDev = !app.isPackaged;

  if (isDev) {
    // Try to load from Vite dev server
    mainWindow.loadURL('http://localhost:5173').catch(() => {
      // Fallback to built file if Vite isn't running
      mainWindow.loadFile(path.join(__dirname, '../dist/index.html')).catch(() => {
        console.error('Failed to load app - neither Vite dev server nor dist/index.html available');
      });
    });
    mainWindow.webContents.openDevTools();
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  startBridgeService();
  createWindow();
});

app.on('window-all-closed', async () => {
  await stopAllServers();
  stopBridgeService();
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

app.on('before-quit', async () => {
  await stopAllServers();
  stopBridgeService();
});

// State for devices/bridges
const state = {
  devices: [],
  bridges: [],
};

// IPC handlers
ipcMain.handle('get-devices', async () => {
  return state.devices;
});

ipcMain.handle('get-bridges', async () => {
  if (bridgeReady) {
    try {
      // We don't have a proper request/response system, so return cached state
      return state.bridges;
    } catch (e) {
      console.error('Failed to list bridges:', e);
    }
  }
  return state.bridges;
});

ipcMain.handle('create-bridge', async (event, config) => {
  console.log('Creating bridge:', config);

  if (bridgeReady) {
    try {
      sendToBridge({
        type: 'create_bridge',
        id: config.id || null,
        source: config.source,
        source_addr: config.sourceAddr,
        target: config.target,
        target_addr: config.targetAddr,
      });

      const bridge = {
        id: config.id || Date.now().toString(),
        source: config.source,
        sourceAddr: config.sourceAddr,
        target: config.target,
        targetAddr: config.targetAddr,
        active: true,
      };
      state.bridges.push(bridge);
      return bridge;
    } catch (e) {
      console.error('Failed to create bridge:', e);
      throw e;
    }
  }

  // Fallback without Rust service
  const bridge = { id: Date.now().toString(), ...config, active: false };
  state.bridges.push(bridge);
  return bridge;
});

ipcMain.handle('delete-bridge', async (event, id) => {
  console.log('Deleting bridge:', id);

  if (bridgeReady) {
    sendToBridge({
      type: 'delete_bridge',
      id: id,
    });
  }

  state.bridges = state.bridges.filter(b => b.id !== id);
  return true;
});

ipcMain.handle('scan-network', async () => {
  mainWindow?.webContents.send('scan-started');

  const portsToScan = [7330, 8080, 9000];
  const hosts = ['localhost', '127.0.0.1'];

  // Get local network hosts
  try {
    const interfaces = os.networkInterfaces();
    for (const iface of Object.values(interfaces)) {
      for (const config of iface) {
        if (config.family === 'IPv4' && !config.internal) {
          const parts = config.address.split('.');
          const subnet = `${parts[0]}.${parts[1]}.${parts[2]}`;
          for (let i = 1; i <= 10; i++) {
            hosts.push(`${subnet}.${i}`);
          }
        }
      }
    }
  } catch (e) {
    console.log('Could not enumerate network interfaces:', e);
  }

  const discoveredDevices = [];
  const probePromises = [];

  for (const host of hosts) {
    for (const port of portsToScan) {
      probePromises.push(probeServer(host, port));
    }
  }

  const results = await Promise.allSettled(probePromises);

  const seen = new Set();
  for (const result of results) {
    if (result.status === 'fulfilled' && result.value) {
      const server = result.value;
      const key = `${server.host}:${server.port}`;
      if (!seen.has(key)) {
        seen.add(key);
        discoveredDevices.push(server);
        mainWindow?.webContents.send('device-found', server);
      }
    }
  }

  for (const device of discoveredDevices) {
    const existing = state.devices.find(d => d.id === device.id);
    if (!existing) {
      state.devices.push(device);
    }
  }

  mainWindow?.webContents.send('scan-complete');
  return discoveredDevices;
});

// Probe a single server
async function probeServer(host, port) {
  return new Promise((resolve) => {
    const wsUrl = `ws://${host}:${port}`;
    const timeout = setTimeout(() => {
      ws.terminate();
      resolve(null);
    }, 2000);

    let ws;
    try {
      ws = new WebSocket(wsUrl, 'clasp.v2');

      ws.on('open', () => {
        clearTimeout(timeout);
        ws.close();
        resolve({
          id: `discovered-${host}-${port}`,
          name: `CLASP Server (${host}:${port})`,
          host,
          port,
          address: wsUrl,
          protocol: 'clasp',
          status: 'available',
        });
      });

      ws.on('error', () => {
        clearTimeout(timeout);
        resolve(null);
      });

      ws.on('close', () => {
        clearTimeout(timeout);
      });
    } catch (e) {
      clearTimeout(timeout);
      resolve(null);
    }
  });
}

ipcMain.handle('add-server', async (event, address) => {
  const server = {
    id: Date.now().toString(),
    name: `Server @ ${address}`,
    address,
    protocol: 'clasp',
    status: 'available',
  };
  state.devices.push(server);
  mainWindow?.webContents.send('device-found', server);
  return server;
});

// Start a server
ipcMain.handle('start-server', async (event, config) => {
  console.log('Starting server:', config);

  const serverType = config.type || config.protocol || 'clasp';
  const serverId = config.id || Date.now().toString();
  config.id = serverId;

  try {
    let result;

    switch (serverType) {
      case 'clasp':
        result = await startClaspServer(config);
        break;
      case 'osc':
        result = await startOscServer(config);
        break;
      case 'mqtt':
        result = await startMqttServer(config);
        break;
      case 'websocket':
        result = await startWebSocketServer(config);
        break;
      case 'http':
        result = await startHttpServer(config);
        break;
      case 'artnet':
        result = await startArtNetServer(config);
        break;
      case 'dmx':
        result = await startDmxServer(config);
        break;
      default:
        throw new Error(`Unknown server type: ${serverType}`);
    }

    return {
      id: serverId,
      status: result.status || 'running',
    };

  } catch (err) {
    console.error('Failed to start server:', err);
    mainWindow?.webContents.send('server-status', {
      id: serverId,
      status: 'error',
      error: err.message,
    });
    throw err;
  }
});

// Stop a server
ipcMain.handle('stop-server', async (event, id) => {
  console.log('Stopping server:', id);

  try {
    const stopped = await stopServer(id);

    // Remove from state
    const idx = state.devices.findIndex(d => d.id === id);
    if (idx !== -1) {
      state.devices.splice(idx, 1);
    }

    mainWindow?.webContents.send('server-status', {
      id: id,
      status: 'stopped',
    });

    return stopped;
  } catch (err) {
    console.error('Failed to stop server:', err);
    throw err;
  }
});

// Get server logs
ipcMain.handle('get-server-logs', async (event, id) => {
  const server = runningServers.get(id);
  if (server) {
    return server.logs;
  }
  return [];
});

// Test connection to a server
ipcMain.handle('test-connection', async (event, address) => {
  return new Promise((resolve) => {
    const wsUrl = address.startsWith('ws://') ? address : `ws://${address}`;
    const timeout = setTimeout(() => {
      ws.terminate();
      resolve({ success: false, error: 'Connection timeout' });
    }, 5000);

    let ws;
    try {
      ws = new WebSocket(wsUrl, 'clasp.v2');

      ws.on('open', () => {
        clearTimeout(timeout);
        ws.close();
        resolve({ success: true });
      });

      ws.on('error', (err) => {
        clearTimeout(timeout);
        resolve({ success: false, error: err.message });
      });
    } catch (e) {
      clearTimeout(timeout);
      resolve({ success: false, error: e.message });
    }
  });
});

// Send a signal via a bridge
ipcMain.handle('send-signal', async (event, { bridgeId, address, value }) => {
  if (bridgeReady) {
    sendToBridge({
      type: 'send_signal',
      bridge_id: bridgeId,
      address,
      value,
    });
    return true;
  }
  return false;
});
