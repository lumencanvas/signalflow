/**
 * CLASP Bridge Configuration Import/Export
 *
 * Handles saving and loading complete bridge configurations
 * including servers, bridges, and mappings.
 */

// Current config format version
const CONFIG_VERSION = 1;

/**
 * Export current configuration to a JSON object
 * @param {Object} state - Application state with servers, bridges, mappings
 * @returns {Object} Exportable configuration object
 */
export function exportConfig(state) {
  return {
    version: CONFIG_VERSION,
    exportedAt: new Date().toISOString(),
    name: state.configName || 'CLASP Bridge Configuration',
    servers: state.servers.map(s => sanitizeServer(s)),
    bridges: state.bridges.map(b => sanitizeBridge(b)),
    mappings: state.mappings.map(m => sanitizeMapping(m)),
  };
}

/**
 * Import configuration from a JSON object
 * @param {Object} config - Configuration to import
 * @returns {Object} Validated configuration object
 */
export function importConfig(config) {
  // Validate version
  if (!config.version) {
    throw new Error('Invalid configuration: missing version');
  }

  if (config.version > CONFIG_VERSION) {
    throw new Error(`Configuration version ${config.version} is newer than supported (${CONFIG_VERSION})`);
  }

  // Validate required fields
  if (!Array.isArray(config.servers)) {
    throw new Error('Invalid configuration: servers must be an array');
  }
  if (!Array.isArray(config.bridges)) {
    throw new Error('Invalid configuration: bridges must be an array');
  }
  if (!Array.isArray(config.mappings)) {
    throw new Error('Invalid configuration: mappings must be an array');
  }

  // Regenerate IDs to avoid conflicts
  const now = Date.now();
  const servers = config.servers.map((s, i) => ({
    ...validateServer(s),
    id: `imported-server-${now}-${i}`,
    status: 'disconnected',
  }));

  const bridges = config.bridges.map((b, i) => ({
    ...validateBridge(b),
    id: `imported-bridge-${now}-${i}`,
    active: false,
  }));

  const mappings = config.mappings.map((m, i) => ({
    ...validateMapping(m),
    id: `imported-mapping-${now}-${i}`,
    enabled: true,
  }));

  return {
    name: config.name || 'Imported Configuration',
    servers,
    bridges,
    mappings,
  };
}

/**
 * Download configuration as a JSON file
 * @param {Object} state - Application state
 * @param {string} filename - Optional filename
 */
export function downloadConfig(state, filename = 'clasp-config.json') {
  const config = exportConfig(state);
  const json = JSON.stringify(config, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);

  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/**
 * Load configuration from a file input
 * @param {File} file - File object from input
 * @returns {Promise<Object>} Parsed and validated configuration
 */
export function loadConfigFromFile(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = (e) => {
      try {
        const json = e.target.result;
        const config = JSON.parse(json);
        const validated = importConfig(config);
        resolve(validated);
      } catch (error) {
        reject(new Error(`Failed to parse configuration: ${error.message}`));
      }
    };

    reader.onerror = () => {
      reject(new Error('Failed to read file'));
    };

    reader.readAsText(file);
  });
}

// Sanitize server config for export (remove runtime data)
function sanitizeServer(server) {
  const {
    id: _id,
    status: _status,
    error: _error,
    process: _process,
    logs: _logs,
    ...config
  } = server;
  return config;
}

// Sanitize bridge config for export
function sanitizeBridge(bridge) {
  const {
    id: _id,
    active: _active,
    ...config
  } = bridge;
  return config;
}

// Sanitize mapping config for export
function sanitizeMapping(mapping) {
  const {
    id: _id,
    enabled: _enabled,
    ...config
  } = mapping;
  return config;
}

// Validate server configuration
function validateServer(server) {
  if (!server.type) {
    throw new Error('Server missing type');
  }

  const validTypes = ['clasp', 'osc', 'midi', 'mqtt', 'websocket', 'http', 'artnet', 'dmx', 'socketio'];
  if (!validTypes.includes(server.type)) {
    throw new Error(`Invalid server type: ${server.type}`);
  }

  return {
    type: server.type,
    protocol: server.protocol || server.type,
    name: server.name || `${server.type.toUpperCase()} Server`,
    address: server.address || '',
    // Protocol-specific fields
    bind: server.bind,
    port: server.port,
    host: server.host,
    topics: server.topics,
    mode: server.mode,
    basePath: server.basePath,
    cors: server.cors,
    subnet: server.subnet,
    universe: server.universe,
    serialPort: server.serialPort,
    announce: server.announce,
    token: server.token,
  };
}

// Validate bridge configuration
function validateBridge(bridge) {
  if (!bridge.source || !bridge.target) {
    throw new Error('Bridge missing source or target');
  }

  return {
    source: bridge.source,
    sourceAddr: bridge.sourceAddr || '',
    target: bridge.target,
    targetAddr: bridge.targetAddr || '',
  };
}

// Validate mapping configuration
function validateMapping(mapping) {
  if (!mapping.source || !mapping.target) {
    throw new Error('Mapping missing source or target');
  }

  return {
    source: mapping.source,
    target: mapping.target,
    transform: mapping.transform || { type: 'direct' },
  };
}

/**
 * Merge imported config with existing state
 * @param {Object} existing - Current state
 * @param {Object} imported - Imported configuration
 * @param {string} mode - 'replace' | 'merge'
 * @returns {Object} Merged state
 */
export function mergeConfig(existing, imported, mode = 'merge') {
  if (mode === 'replace') {
    return {
      servers: imported.servers,
      bridges: imported.bridges,
      mappings: imported.mappings,
    };
  }

  // Merge mode - add to existing
  return {
    servers: [...existing.servers, ...imported.servers],
    bridges: [...existing.bridges, ...imported.bridges],
    mappings: [...existing.mappings, ...imported.mappings],
  };
}
