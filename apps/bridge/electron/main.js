const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const { spawn, execSync } = require('child_process');
const readline = require('readline');
const WebSocket = require('ws');
const os = require('os');
const { SerialPort } = require('serialport');

let mainWindow;
let bridgeService = null;
let bridgeReady = false;

// Track running server processes
const runningServers = new Map(); // id -> { process, config, status, logs, stats }
const MAX_LOG_LINES = 500;

// Stats tracking interval
let statsInterval = null;

// Initialize stats for a server
function createServerStats() {
  return {
    startTime: Date.now(),
    messagesIn: 0,
    messagesOut: 0,
    bytesIn: 0,
    bytesOut: 0,
    errors: 0,
    connections: 0,
    lastActivity: null,
    lastError: null,
  };
}

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
        // Non-JSON output from bridge service (logs)
      }
    });

    bridgeService.stderr.on('data', (data) => {
      // Bridge service stderr output
    });

    bridgeService.on('close', (code) => {
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
      bridgeReady = true;
      break;

    case 'signal':
      // Update stats for the source server and get server metadata
      let serverMeta = null;
      if (message.bridge_id) {
        const server = runningServers.get(message.bridge_id);
        if (server) {
          if (server.stats) {
            server.stats.messagesIn++;
            server.stats.lastActivity = Date.now();
          }
          // Extract metadata for the signal
          serverMeta = {
            protocol: server.config?.type || 'unknown',
            serverName: server.config?.name || server.config?.claspName || server.config?.type?.toUpperCase() || 'Unknown',
            port: server.port || server.config?.port || null,
            address: server.config?.address || server.config?.claspAddress || null,
          };
        }
      }

      // Forward signal to renderer with enriched metadata
      if (mainWindow) {
        mainWindow.webContents.send('signal', {
          bridgeId: message.bridge_id,
          address: message.address,
          value: message.value,
          // Enriched metadata
          protocol: serverMeta?.protocol || message.protocol || 'unknown',
          serverName: serverMeta?.serverName || null,
          serverPort: serverMeta?.port || null,
          serverAddress: serverMeta?.address || null,
        });
      }
      break;

    case 'bridge_event':
      // Update stats based on event type
      if (message.bridge_id) {
        const server = runningServers.get(message.bridge_id);
        if (server && server.stats) {
          if (message.event === 'connected') {
            server.stats.connections++;
          } else if (message.event === 'error') {
            server.stats.errors++;
            server.stats.lastError = Date.now();
          }
          server.stats.lastActivity = Date.now();
        }
      }

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

  const args = [
    '--listen', `${host === 'localhost' ? '0.0.0.0' : host}:${port}`,
    '--name', config.name || 'CLASP Bridge Server',
  ];

  if (config.announce !== false) {
    args.push('--announce');
  }

  // Add security configuration
  let tokenFilePath = null;
  if (config.authEnabled && config.tokenFileContent) {
    // Write token file content to a temp file
    const tokensDir = path.join(app.getPath('userData'), 'tokens');
    if (!fs.existsSync(tokensDir)) {
      fs.mkdirSync(tokensDir, { recursive: true });
    }
    tokenFilePath = path.join(tokensDir, `tokens-${config.id}.txt`);
    fs.writeFileSync(tokenFilePath, config.tokenFileContent, 'utf8');

    args.push('--auth-mode', 'authenticated');
    args.push('--token-file', tokenFilePath);
  } else if (config.token) {
    // Fallback to single token (backwards compatibility)
    args.push('--auth-mode', 'authenticated');
    args.push('--token', config.token);
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
        stats: createServerStats(),
        tokenFilePath, // Store for cleanup
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
      setTimeout(async () => {
        if (serverState.status === 'starting' && proc.exitCode === null) {
          serverState.status = 'running';
          mainWindow?.webContents.send('server-status', {
            id: config.id,
            status: 'running',
          });
        }

        // Create a monitor connection to observe CLASP traffic
        try {
          await createClaspMonitor(config.id, `ws://127.0.0.1:${port}`, config.token);
        } catch (err) {
          // CLASP monitor connection failed - non-critical
        }

        resolve({ id: config.id, status: serverState.status });
      }, 500);

    } catch (err) {
      reject(err);
    }
  });
}

// CLASP monitor connections
const claspMonitors = new Map();

// CLASP message type strings (matching Rust serde tags)
const MSG = {
  HELLO: 'HELLO',
  WELCOME: 'WELCOME',
  SUBSCRIBE: 'SUBSCRIBE',
  UNSUBSCRIBE: 'UNSUBSCRIBE',
  SET: 'SET',
  PUBLISH: 'PUBLISH',
  SNAPSHOT: 'SNAPSHOT',
  PING: 'PING',
  PONG: 'PONG',
  ACK: 'ACK',
  ERROR: 'ERROR',
};

// Encode a CLASP frame
function encodeClaspFrame(message) {
  const { encode } = require('@msgpack/msgpack');
  const payload = Buffer.from(encode(message));

  const frame = Buffer.alloc(4 + payload.length);
  frame[0] = 0x53;  // Magic 'S' (for Streaming)
  frame[1] = 0x00;  // Flags (QoS=0, no timestamp)
  frame.writeUInt16BE(payload.length, 2);
  payload.copy(frame, 4);

  return frame;
}

// Decode a CLASP frame
function decodeClaspFrame(buffer) {
  const { decode } = require('@msgpack/msgpack');

  if (buffer[0] !== 0x53) {
    throw new Error(`Invalid magic byte: expected 0x53, got 0x${buffer[0].toString(16)}`);
  }

  const flags = buffer[1];
  const hasTimestamp = (flags & 0x20) !== 0;
  const payloadLength = buffer.readUInt16BE(2);

  let payloadOffset = hasTimestamp ? 12 : 4;
  const payload = buffer.slice(payloadOffset, payloadOffset + payloadLength);
  const message = decode(payload);

  return message;
}

// Create a WebSocket monitor connection to observe CLASP traffic
async function createClaspMonitor(serverId, wsUrl, token = null) {
  // Close existing monitor if any
  if (claspMonitors.has(serverId)) {
    try {
      claspMonitors.get(serverId).close();
    } catch (e) {}
  }

  return new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl, 'clasp');
    ws.binaryType = 'nodebuffer';
    let connected = false;
    let welcomed = false;

    ws.on('open', () => {
      connected = true;
      claspMonitors.set(serverId, ws);

      // Send HELLO message with optional token
      const helloMsg = {
        type: MSG.HELLO,
        version: 2,
        name: 'CLASP Bridge Monitor',
        features: ['param', 'event', 'stream'],
      };
      if (token) {
        helloMsg.token = token;
      }
      const hello = encodeClaspFrame(helloMsg);
      ws.send(hello);
    });

    ws.on('message', (data) => {
      try {
        const buffer = Buffer.from(data);
        const msg = decodeClaspFrame(buffer);

        // Handle WELCOME - send subscribe for all addresses
        if (msg.type === MSG.WELCOME) {
          welcomed = true;

          // Subscribe to all signals
          const subscribe = encodeClaspFrame({
            type: MSG.SUBSCRIBE,
            id: 1,
            pattern: '/**',  // Subscribe to all addresses
            types: ['param', 'event', 'stream'],
          });
          ws.send(subscribe);
          resolve(ws);
          return;
        }

        // Handle ERROR - authentication or authorization failure
        if (msg.type === MSG.ERROR) {
          const errorCode = msg.code || 0;
          const errorMessage = msg.message || 'Unknown error';

          // 300 = Unauthorized (no token), 301 = Forbidden (bad scopes), 302 = Token expired
          if (errorCode >= 300 && errorCode < 400) {
            ws.close();
            claspMonitors.delete(serverId);

            // Notify renderer about auth failure
            mainWindow?.webContents.send('server-status', {
              id: serverId,
              status: 'error',
              error: `Authentication failed: ${errorMessage}`,
            });

            reject(new Error(`Authentication failed: ${errorMessage}`));
            return;
          }

          // Log other errors but don't disconnect
          console.error(`CLASP error from server ${serverId}: ${errorCode} - ${errorMessage}`);
          return;
        }

        // Handle PING
        if (msg.type === MSG.PING) {
          ws.send(encodeClaspFrame({ type: MSG.PONG }));
          return;
        }

        // Handle SET, PUBLISH, and SNAPSHOT messages
        if (msg.type === MSG.SET || msg.type === MSG.PUBLISH) {
          const serverInfo = runningServers.get(serverId);
          if (serverInfo && serverInfo.stats) {
            serverInfo.stats.messagesIn++;
            serverInfo.stats.lastActivity = Date.now();
          }

          const signal = {
            bridgeId: serverId,
            address: msg.address || '/',
            value: msg.value !== undefined ? msg.value : msg.payload,
            protocol: 'clasp',
            serverName: serverInfo?.config?.name || 'CLASP Server',
            serverPort: serverInfo?.port,
          };

          // Forward to renderer as a signal
          mainWindow?.webContents.send('signal', signal);

          // If learn mode is active, also send as learned signal
          if (learnModeActive && learnModeTarget) {
            mainWindow?.webContents.send('learned-signal', {
              ...signal,
              target: learnModeTarget,
            });
          }
        }

        // Handle SNAPSHOT (initial state dump)
        if (msg.type === MSG.SNAPSHOT && msg.params) {
          const serverInfo = runningServers.get(serverId);
          for (const param of msg.params) {
            mainWindow?.webContents.send('signal', {
              bridgeId: serverId,
              address: param.address,
              value: param.value,
              protocol: 'clasp',
              serverName: serverInfo?.config?.name || 'CLASP Server',
              serverPort: serverInfo?.port,
            });
          }
        }
      } catch (e) {
        // Decode error - silently ignore malformed messages
      }
    });

    ws.on('error', (err) => {
      if (!connected) {
        reject(err);
      }
    });

    ws.on('close', (code, reason) => {
      claspMonitors.delete(serverId);
    });

    ws.on('unexpected-response', (req, res) => {
      // Unexpected HTTP response - connection will fail
    });

    // Timeout for initial connection
    setTimeout(() => {
      if (!connected) {
        ws.terminate();
        reject(new Error('Connection timeout'));
      }
    }, 5000);
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
    stats: createServerStats(),
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
    stats: createServerStats(),
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
    stats: createServerStats(),
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
    stats: createServerStats(),
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
    stats: createServerStats(),
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
    stats: createServerStats(),
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

  // Close CLASP monitor if exists
  if (claspMonitors.has(id)) {
    try {
      claspMonitors.get(id).close();
      claspMonitors.delete(id);
    } catch (e) {
      // ignore
    }
  }

  if (server.process) {
    // For process-based servers (CLASP router)
    server.process.kill('SIGTERM');
    // Give it a moment to shut down gracefully
    await new Promise(resolve => setTimeout(resolve, 500));
    if (server.process && server.process.exitCode === null) {
      server.process.kill('SIGKILL');
    }

    // Clean up token file if it exists
    if (server.tokenFilePath) {
      try {
        fs.unlinkSync(server.tokenFilePath);
      } catch (e) {
        // Ignore cleanup errors
      }
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
    startStatsBroadcast();
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
    stopStatsBroadcast();
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

// Use will-quit for synchronous cleanup since before-quit doesn't properly await
app.on('will-quit', (event) => {
  // Stop stats broadcast immediately
  stopStatsBroadcast();

  // Close all CLASP monitors
  for (const [id, ws] of claspMonitors) {
    try {
      ws.close();
    } catch (e) { /* ignore */ }
  }
  claspMonitors.clear();

  // Stop bridge service synchronously
  stopBridgeService();
});

// Also handle before-quit for async cleanup with a short timeout
app.on('before-quit', async (event) => {
  event.preventDefault();
  try {
    await Promise.race([
      stopAllServers(),
      new Promise(resolve => setTimeout(resolve, 2000)), // 2s timeout
    ]);
  } catch (e) { /* ignore cleanup errors */ }
  app.exit(0);
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
    // Network interface enumeration failed - continue with defaults
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
    let ws;

    const timeout = setTimeout(() => {
      if (ws) ws.terminate();
      resolve(null);
    }, 2000);

    try {
      ws = new WebSocket(wsUrl, 'clasp');

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
    let ws;
    let timeout;

    try {
      ws = new WebSocket(wsUrl, 'clasp');

      timeout = setTimeout(() => {
        if (ws) ws.terminate();
        resolve({ success: false, error: 'Connection timeout' });
      }, 5000);

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
      if (timeout) clearTimeout(timeout);
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

// ============================================
// Learn Mode
// ============================================

let learnModeActive = false;
let learnModeTarget = null;

ipcMain.handle('start-learn-mode', async (event, target) => {
  learnModeActive = true;
  learnModeTarget = target;
  return true;
});

ipcMain.handle('stop-learn-mode', async () => {
  learnModeActive = false;
  learnModeTarget = null;
  return true;
});

// ============================================
// Configuration & App Info
// ============================================

const configPath = path.join(app.getPath('userData'), 'clasp-config.json');

ipcMain.handle('get-app-version', () => {
  return app.getVersion();
});

ipcMain.handle('is-first-run', () => {
  try {
    if (!fs.existsSync(configPath)) {
      return true;
    }
    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    return !config.firstRunComplete;
  } catch (e) {
    return true;
  }
});

ipcMain.handle('set-first-run-complete', () => {
  try {
    let config = {};
    if (fs.existsSync(configPath)) {
      config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    }
    config.firstRunComplete = true;
    config.firstRunDate = new Date().toISOString();
    fs.writeFileSync(configPath, JSON.stringify(config, null, 2));
    return true;
  } catch (e) {
    console.error('Failed to save first run state:', e);
    return false;
  }
});

// ============================================
// File Dialogs (for config import/export)
// ============================================

ipcMain.handle('show-save-dialog', async (event, options) => {
  const result = await dialog.showSaveDialog(mainWindow, {
    title: options.title || 'Save Configuration',
    defaultPath: options.defaultPath || 'clasp-config.json',
    filters: options.filters || [
      { name: 'JSON Files', extensions: ['json'] },
      { name: 'All Files', extensions: ['*'] },
    ],
  });
  return result;
});

ipcMain.handle('show-open-dialog', async (event, options) => {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: options.title || 'Load Configuration',
    filters: options.filters || [
      { name: 'JSON Files', extensions: ['json'] },
      { name: 'All Files', extensions: ['*'] },
    ],
    properties: ['openFile'],
  });
  return result;
});

// Validate file path is within allowed directories
function isPathAllowed(filePath) {
  const resolvedPath = path.resolve(filePath);
  const allowedDirs = [
    app.getPath('userData'),
    app.getPath('documents'),
    app.getPath('downloads'),
    app.getPath('home'),
  ];
  // Allow if path is within any allowed directory
  return allowedDirs.some(dir => resolvedPath.startsWith(dir));
}

ipcMain.handle('write-file', async (event, { path: filePath, content }) => {
  try {
    if (!isPathAllowed(filePath)) {
      return { success: false, error: 'Access denied: path not in allowed directories' };
    }
    fs.writeFileSync(filePath, content, 'utf8');
    return { success: true };
  } catch (e) {
    return { success: false, error: e.message };
  }
});

ipcMain.handle('read-file', async (event, filePath) => {
  try {
    if (!isPathAllowed(filePath)) {
      return { success: false, error: 'Access denied: path not in allowed directories' };
    }
    const content = fs.readFileSync(filePath, 'utf8');
    return { success: true, content };
  } catch (e) {
    return { success: false, error: e.message };
  }
});

// ============================================
// Hardware Discovery
// ============================================

// List serial ports (for DMX interfaces)
ipcMain.handle('list-serial-ports', async () => {
  try {
    const ports = await SerialPort.list();
    // Filter to likely DMX devices (USB serial adapters)
    return ports.map((port) => ({
      path: port.path,
      manufacturer: port.manufacturer || 'Unknown',
      serialNumber: port.serialNumber,
      vendorId: port.vendorId,
      productId: port.productId,
      // Friendly name for UI
      name: port.manufacturer
        ? `${port.manufacturer} (${port.path})`
        : port.path,
    }));
  } catch (e) {
    console.error('Failed to list serial ports:', e);
    return [];
  }
});

// List MIDI ports (via system command)
ipcMain.handle('list-midi-ports', async () => {
  const ports = { inputs: [], outputs: [] };

  try {
    if (process.platform === 'darwin') {
      // macOS: Use system_profiler or ioreg
      try {
        // Try to get MIDI devices via Audio MIDI Setup info
        const output = execSync(
          'system_profiler SPMIDIDataType -json 2>/dev/null || echo "{}"',
          { encoding: 'utf8', timeout: 5000 }
        );
        const data = JSON.parse(output);
        if (data.SPMIDIDataType) {
          for (const device of data.SPMIDIDataType) {
            if (device._name) {
              // Add as both input and output (we can't distinguish easily)
              ports.inputs.push({
                id: device._name,
                name: device._name,
                manufacturer: device.manufacturer || 'Unknown',
              });
              ports.outputs.push({
                id: device._name,
                name: device._name,
                manufacturer: device.manufacturer || 'Unknown',
              });
            }
          }
        }
      } catch (e) {
        // Fallback: Check common MIDI locations
        const commonPorts = [
          'IAC Driver Bus 1',
          'Network Session 1',
        ];
        for (const name of commonPorts) {
          ports.inputs.push({ id: name, name, manufacturer: 'System' });
          ports.outputs.push({ id: name, name, manufacturer: 'System' });
        }
      }
    } else if (process.platform === 'linux') {
      // Linux: Parse /proc/asound/seq/clients or use aconnect
      try {
        const output = execSync('aconnect -l 2>/dev/null || echo ""', {
          encoding: 'utf8',
          timeout: 5000,
        });
        const lines = output.split('\n');
        for (const line of lines) {
          const match = line.match(/client (\d+): '([^']+)'/);
          if (match) {
            const [, id, name] = match;
            if (name !== 'System' && name !== 'Midi Through') {
              ports.inputs.push({ id, name, manufacturer: 'ALSA' });
              ports.outputs.push({ id, name, manufacturer: 'ALSA' });
            }
          }
        }
      } catch (e) {
        // MIDI enumeration not available
      }
    } else if (process.platform === 'win32') {
      // Windows: Use powershell or midiInGetNumDevs via ffi
      // For now, just return common names
      ports.inputs.push({
        id: 'default',
        name: 'Default MIDI Input',
        manufacturer: 'System',
      });
      ports.outputs.push({
        id: 'default',
        name: 'Default MIDI Output',
        manufacturer: 'System',
      });
    }
  } catch (e) {
    console.error('Failed to enumerate MIDI ports:', e);
  }

  // Always include "default" option
  if (!ports.inputs.find((p) => p.id === 'default')) {
    ports.inputs.unshift({
      id: 'default',
      name: 'System Default',
      manufacturer: 'System',
    });
  }
  if (!ports.outputs.find((p) => p.id === 'default')) {
    ports.outputs.unshift({
      id: 'default',
      name: 'System Default',
      manufacturer: 'System',
    });
  }

  return ports;
});

// List network interfaces (for binding servers)
ipcMain.handle('list-network-interfaces', async () => {
  const interfaces = [];

  try {
    const netInterfaces = os.networkInterfaces();
    for (const [name, addrs] of Object.entries(netInterfaces)) {
      for (const addr of addrs) {
        if (addr.family === 'IPv4') {
          interfaces.push({
            name,
            address: addr.address,
            internal: addr.internal,
            label: addr.internal
              ? `${addr.address} (${name} - loopback)`
              : `${addr.address} (${name})`,
          });
        }
      }
    }
  } catch (e) {
    console.error('Failed to list network interfaces:', e);
  }

  // Always include 0.0.0.0 (all interfaces)
  interfaces.unshift({
    name: 'all',
    address: '0.0.0.0',
    internal: false,
    label: '0.0.0.0 (All Interfaces)',
  });

  return interfaces;
});

// Test a serial port connection
ipcMain.handle('test-serial-port', async (event, portPath) => {
  return new Promise((resolve) => {
    try {
      const port = new SerialPort({
        path: portPath,
        baudRate: 250000, // DMX baud rate
        autoOpen: false,
      });

      port.open((err) => {
        if (err) {
          resolve({ success: false, error: err.message });
        } else {
          port.close();
          resolve({ success: true });
        }
      });

      // Timeout after 3 seconds
      setTimeout(() => {
        try {
          port.close();
        } catch (e) {
          // ignore
        }
        resolve({ success: false, error: 'Connection timeout' });
      }, 3000);
    } catch (e) {
      resolve({ success: false, error: e.message });
    }
  });
});

// Test OSC port availability
ipcMain.handle('test-port-available', async (event, { host, port }) => {
  return new Promise((resolve) => {
    const dgram = require('dgram');
    const socket = dgram.createSocket('udp4');

    socket.on('error', (err) => {
      socket.close();
      resolve({ success: false, error: err.message });
    });

    socket.bind(port, host, () => {
      socket.close();
      resolve({ success: true });
    });

    // Timeout
    setTimeout(() => {
      try {
        socket.close();
      } catch (e) {
        // ignore
      }
      resolve({ success: false, error: 'Timeout' });
    }, 2000);
  });
});

// ============================================
// Server Stats & Diagnostics
// ============================================

// Get detailed stats for a server
ipcMain.handle('get-server-stats', async (event, id) => {
  const server = runningServers.get(id);
  if (!server) {
    return null;
  }

  const stats = server.stats || {};
  const uptime = stats.startTime ? Date.now() - stats.startTime : 0;

  return {
    id,
    status: server.status,
    uptime,
    uptimeFormatted: formatUptime(uptime),
    messagesIn: stats.messagesIn || 0,
    messagesOut: stats.messagesOut || 0,
    bytesIn: stats.bytesIn || 0,
    bytesOut: stats.bytesOut || 0,
    errors: stats.errors || 0,
    connections: stats.connections || 0,
    lastActivity: stats.lastActivity,
    lastError: stats.lastError,
    config: server.config,
    port: server.port,
  };
});

// Get stats for all running servers
ipcMain.handle('get-all-server-stats', async () => {
  const allStats = [];
  for (const [id, server] of runningServers) {
    const stats = server.stats || {};
    const uptime = stats.startTime ? Date.now() - stats.startTime : 0;
    allStats.push({
      id,
      status: server.status,
      uptime,
      uptimeFormatted: formatUptime(uptime),
      messagesIn: stats.messagesIn || 0,
      messagesOut: stats.messagesOut || 0,
      errors: stats.errors || 0,
      connections: stats.connections || 0,
      lastActivity: stats.lastActivity,
      protocol: server.config?.protocol || server.config?.type,
      name: server.config?.name,
    });
  }
  return allStats;
});

// Format uptime as human-readable string
function formatUptime(ms) {
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
// Test Signal Generator
// ============================================

// Send a test signal through the bridge
ipcMain.handle('send-test-signal', async (event, { protocol, address, signalAddress, value }) => {
  if (!bridgeReady) {
    return { success: false, error: 'Bridge service not ready' };
  }

  try {
    // Send test signal via bridge service
    sendToBridge({
      type: 'send_signal',
      protocol,
      target_addr: address,
      signal: {
        address: signalAddress,
        value,
      },
    });

    // Update stats
    for (const [id, server] of runningServers) {
      if (server.config?.address === address || server.config?.bind === address) {
        if (server.stats) {
          server.stats.messagesOut++;
          server.stats.lastActivity = Date.now();
        }
      }
    }

    return { success: true };
  } catch (e) {
    return { success: false, error: e.message };
  }
});

// Send a batch of test signals
ipcMain.handle('send-test-signal-batch', async (event, { signals }) => {
  if (!bridgeReady) {
    return { success: false, error: 'Bridge service not ready' };
  }

  let sent = 0;
  for (const signal of signals) {
    try {
      sendToBridge({
        type: 'send_signal',
        protocol: signal.protocol,
        target_addr: signal.address,
        signal: {
          address: signal.signalAddress,
          value: signal.value,
        },
      });
      sent++;
    } catch (e) {
      console.error('Failed to send test signal:', e);
    }
  }

  return { success: true, sent };
});

// ============================================
// Health Check & Diagnostics
// ============================================

// Run health check on a server
ipcMain.handle('health-check', async (event, id) => {
  const server = runningServers.get(id);
  if (!server) {
    return { healthy: false, error: 'Server not found' };
  }

  const checks = {
    processRunning: false,
    portOpen: false,
    lastActivityRecent: false,
    noRecentErrors: true,
  };

  // Check if process is running (for CLASP router)
  if (server.process) {
    checks.processRunning = server.process.exitCode === null;
  } else {
    // Bridge-based servers are considered running if status is 'running'
    checks.processRunning = server.status === 'running';
  }

  // Check if port is accepting connections (for TCP-based protocols)
  if (server.port && server.config?.type !== 'dmx') {
    try {
      const net = require('net');
      checks.portOpen = await new Promise((resolve) => {
        const socket = new net.Socket();
        socket.setTimeout(2000);
        socket.on('connect', () => {
          socket.destroy();
          resolve(true);
        });
        socket.on('error', () => resolve(false));
        socket.on('timeout', () => {
          socket.destroy();
          resolve(false);
        });
        socket.connect(server.port, '127.0.0.1');
      });
    } catch (e) {
      checks.portOpen = false;
    }
  } else {
    checks.portOpen = true; // Skip for DMX/non-port servers
  }

  // Check last activity
  const stats = server.stats || {};
  if (stats.lastActivity) {
    const timeSinceActivity = Date.now() - stats.lastActivity;
    checks.lastActivityRecent = timeSinceActivity < 60000; // Within last minute
  }

  // Check recent errors
  checks.noRecentErrors = !stats.lastError || (Date.now() - stats.lastError > 60000);

  const healthy = checks.processRunning && checks.portOpen;

  return {
    healthy,
    checks,
    status: server.status,
    uptime: stats.startTime ? Date.now() - stats.startTime : 0,
  };
});

// Run diagnostics on the entire system
ipcMain.handle('run-diagnostics', async () => {
  const diagnostics = {
    bridgeService: {
      running: bridgeService !== null && bridgeReady,
      pid: bridgeService?.pid,
    },
    servers: [],
    system: {
      platform: process.platform,
      nodeVersion: process.version,
      electronVersion: process.versions.electron,
      memoryUsage: process.memoryUsage(),
      uptime: process.uptime(),
    },
  };

  // Check each server
  for (const [id, server] of runningServers) {
    const stats = server.stats || {};
    diagnostics.servers.push({
      id,
      name: server.config?.name,
      type: server.config?.type || server.config?.protocol,
      status: server.status,
      processRunning: server.process ? server.process.exitCode === null : true,
      port: server.port,
      uptime: stats.startTime ? Date.now() - stats.startTime : 0,
      messagesIn: stats.messagesIn || 0,
      messagesOut: stats.messagesOut || 0,
      errors: stats.errors || 0,
      lastActivity: stats.lastActivity,
      lastError: stats.lastError,
    });
  }

  return diagnostics;
});

// Start periodic stats broadcast (call when window is ready)
function startStatsBroadcast() {
  if (statsInterval) {
    clearInterval(statsInterval);
  }

  statsInterval = setInterval(() => {
    if (!mainWindow) return;

    const allStats = [];
    for (const [id, server] of runningServers) {
      const stats = server.stats || {};
      allStats.push({
        id,
        status: server.status,
        messagesIn: stats.messagesIn || 0,
        messagesOut: stats.messagesOut || 0,
        errors: stats.errors || 0,
        connections: stats.connections || 0,
        lastActivity: stats.lastActivity,
      });
    }

    mainWindow.webContents.send('server-stats-update', allStats);
  }, 1000); // Update every second
}

function stopStatsBroadcast() {
  if (statsInterval) {
    clearInterval(statsInterval);
    statsInterval = null;
  }
}
