const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const readline = require('readline');

let mainWindow;
let bridgeService = null;
let bridgeReady = false;
let pendingRequests = new Map();
let requestId = 0;

// Path to the Rust bridge service binary
const getBridgeServicePath = () => {
  // In development, use the cargo build output
  // In production, it would be bundled with the app
  const devPath = path.join(__dirname, '..', '..', '..', 'target', 'release', 'clasp-service');
  return devPath;
};

// Start the bridge service
function startBridgeService() {
  const servicePath = getBridgeServicePath();
  console.log('Starting bridge service:', servicePath);

  try {
    bridgeService = spawn(servicePath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    // Handle stdout - JSON messages from the service
    const rl = readline.createInterface({
      input: bridgeService.stdout,
      crlfDelay: Infinity,
    });

    rl.on('line', (line) => {
      try {
        const message = JSON.parse(line);
        handleBridgeMessage(message);
      } catch (e) {
        console.error('Failed to parse bridge message:', e, line);
      }
    });

    // Handle stderr - logging from the service
    bridgeService.stderr.on('data', (data) => {
      console.log('[bridge-service]', data.toString().trim());
    });

    // Handle process exit
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

    case 'ok':
    case 'error':
      // Response to a request - handled via IPC
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

// Send request and wait for response
async function sendRequest(request) {
  return new Promise((resolve, reject) => {
    if (!bridgeService || !bridgeReady) {
      reject(new Error('Bridge service not ready'));
      return;
    }

    // Create a one-time listener for the response
    const responseHandler = (line) => {
      try {
        const message = JSON.parse(line);
        if (message.type === 'ok') {
          resolve(message.data);
        } else if (message.type === 'error') {
          reject(new Error(message.message));
        }
      } catch (e) {
        // Ignore parse errors for non-response messages
      }
    };

    // Listen for response on the line reader
    const rl = readline.createInterface({
      input: bridgeService.stdout,
      crlfDelay: Infinity,
    });

    const timeout = setTimeout(() => {
      rl.close();
      reject(new Error('Request timeout'));
    }, 10000);

    rl.once('line', (line) => {
      clearTimeout(timeout);
      rl.close();
      responseHandler(line);
    });

    // Send the request
    sendToBridge(request);
  });
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
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

  // Load the app
  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL('http://localhost:5173');
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

app.on('window-all-closed', () => {
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

app.on('before-quit', () => {
  stopBridgeService();
});

// State for when bridge service isn't available
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
      const result = await sendRequest({ type: 'list_bridges' });
      return result || [];
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
      // Send to Rust service
      sendToBridge({
        type: 'create_bridge',
        id: config.id || null,
        source: config.source,
        source_addr: config.sourceAddr,
        target: config.target,
        target_addr: config.targetAddr,
      });

      // Return optimistically
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

  // Simulate scan for now - real discovery would use mDNS
  setTimeout(() => {
    mainWindow?.webContents.send('scan-complete');
  }, 1500);

  return state.devices;
});

ipcMain.handle('add-server', async (event, address) => {
  const server = {
    id: Date.now().toString(),
    name: `Server @ ${address}`,
    address,
    protocol: 'clasp',
    status: 'connecting',
  };
  state.devices.push(server);
  mainWindow?.webContents.send('device-found', server);

  // Simulate connection
  setTimeout(() => {
    server.status = 'connected';
    mainWindow?.webContents.send('device-updated', server);
  }, 500);

  return server;
});

ipcMain.handle('start-server', async (event, config) => {
  console.log('Starting server:', config);

  const server = {
    id: config.id || Date.now().toString(),
    ...config,
    status: 'connecting',
  };

  // If backend is ready, try to start the actual server
  if (bridgeReady) {
    try {
      sendToBridge({
        type: 'start_server',
        id: server.id,
        protocol: config.type || config.protocol,
        config: config,
      });
    } catch (e) {
      console.error('Failed to start server via bridge:', e);
    }
  }

  // Simulate successful start
  setTimeout(() => {
    server.status = 'connected';
    mainWindow?.webContents.send('device-updated', server);
  }, 300);

  return server;
});

ipcMain.handle('stop-server', async (event, id) => {
  console.log('Stopping server:', id);

  // Remove from state
  const idx = state.devices.findIndex(d => d.id === id);
  if (idx !== -1) {
    state.devices.splice(idx, 1);
  }

  // Tell backend to stop if ready
  if (bridgeReady) {
    sendToBridge({
      type: 'stop_server',
      id: id,
    });
  }

  mainWindow?.webContents.send('device-lost', id);
  return true;
});
