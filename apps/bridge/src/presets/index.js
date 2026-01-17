/**
 * CLASP Bridge Workflow Presets
 *
 * Pre-configured setups for common creative workflows.
 * Each preset includes servers, bridges, and optional mappings.
 */

export const presets = [
  {
    id: 'vj-setup',
    name: 'VJ Setup',
    description: 'TouchOSC/Lemur → Resolume/VDMX',
    icon: 'video',
    category: 'visual',
    tags: ['vj', 'resolume', 'vdmx', 'touchosc', 'osc'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
      {
        type: 'osc',
        name: 'OSC Input (TouchOSC)',
        bind: '0.0.0.0',
        port: 9000,
      },
    ],
    bridges: [
      {
        source: 'osc',
        sourceAddr: '0.0.0.0:9000',
        target: 'clasp',
        targetAddr: 'internal',
      },
    ],
    mappings: [],
  },

  {
    id: 'lighting-console',
    name: 'Lighting Console',
    description: 'OSC/MIDI → Art-Net + DMX for lighting control',
    icon: 'lightbulb',
    category: 'lighting',
    tags: ['lighting', 'artnet', 'dmx', 'osc', 'midi'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
      {
        type: 'osc',
        name: 'OSC Control Input',
        bind: '0.0.0.0',
        port: 8000,
      },
      {
        type: 'artnet',
        name: 'Art-Net Output',
        bind: '0.0.0.0:6454',
        subnet: 0,
        universe: 0,
      },
    ],
    bridges: [
      {
        source: 'osc',
        sourceAddr: '0.0.0.0:8000',
        target: 'clasp',
        targetAddr: 'internal',
      },
      {
        source: 'clasp',
        sourceAddr: 'internal',
        target: 'artnet',
        targetAddr: '255.255.255.255:6454',
      },
    ],
    mappings: [],
  },

  {
    id: 'midi-hub',
    name: 'MIDI Hub',
    description: 'MIDI → OSC + WebSocket for web apps',
    icon: 'music',
    category: 'audio',
    tags: ['midi', 'osc', 'websocket', 'ableton', 'max'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
      {
        type: 'websocket',
        name: 'WebSocket Server',
        address: '0.0.0.0:8080',
        mode: 'server',
      },
    ],
    bridges: [
      {
        source: 'midi',
        sourceAddr: 'default',
        target: 'clasp',
        targetAddr: 'internal',
      },
      {
        source: 'clasp',
        sourceAddr: 'internal',
        target: 'websocket',
        targetAddr: '0.0.0.0:8080',
      },
    ],
    mappings: [],
  },

  {
    id: 'sensor-network',
    name: 'Sensor Network',
    description: 'MQTT IoT sensors → All protocols',
    icon: 'cpu',
    category: 'iot',
    tags: ['mqtt', 'iot', 'sensors', 'arduino', 'esp32'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
      {
        type: 'mqtt',
        name: 'MQTT Broker Connection',
        host: 'localhost',
        port: 1883,
        topics: ['sensors/#', 'devices/#'],
      },
    ],
    bridges: [
      {
        source: 'mqtt',
        sourceAddr: 'localhost:1883',
        target: 'clasp',
        targetAddr: 'internal',
      },
    ],
    mappings: [],
  },

  {
    id: 'web-control',
    name: 'Web Control',
    description: 'WebSocket + HTTP API for web interfaces',
    icon: 'globe',
    category: 'web',
    tags: ['websocket', 'http', 'api', 'web', 'json'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
      {
        type: 'websocket',
        name: 'WebSocket Server',
        address: '0.0.0.0:8080',
        mode: 'server',
      },
      {
        type: 'http',
        name: 'HTTP REST API',
        bind: '0.0.0.0:3000',
        basePath: '/api',
        cors: true,
      },
    ],
    bridges: [
      {
        source: 'websocket',
        sourceAddr: '0.0.0.0:8080',
        target: 'clasp',
        targetAddr: 'internal',
      },
      {
        source: 'http',
        sourceAddr: '0.0.0.0:3000',
        target: 'clasp',
        targetAddr: 'internal',
      },
    ],
    mappings: [],
  },

  {
    id: 'minimal',
    name: 'Minimal Setup',
    description: 'Just a CLASP server for basic bridging',
    icon: 'zap',
    category: 'basic',
    tags: ['minimal', 'basic', 'simple'],
    servers: [
      {
        type: 'clasp',
        name: 'CLASP Bridge Server',
        address: '0.0.0.0:7330',
        announce: true,
      },
    ],
    bridges: [],
    mappings: [],
  },
];

// Category metadata for UI grouping
export const categories = {
  visual: { name: 'Visual/VJ', color: '#8b5cf6' },
  lighting: { name: 'Lighting', color: '#f59e0b' },
  audio: { name: 'Audio/MIDI', color: '#ec4899' },
  iot: { name: 'IoT/Sensors', color: '#22c55e' },
  web: { name: 'Web/API', color: '#3b82f6' },
  basic: { name: 'Basic', color: '#78716c' },
};

// Get preset by ID
export function getPreset(id) {
  return presets.find(p => p.id === id);
}

// Search presets by tag or name
export function searchPresets(query) {
  const q = query.toLowerCase();
  return presets.filter(p =>
    p.name.toLowerCase().includes(q) ||
    p.description.toLowerCase().includes(q) ||
    p.tags.some(t => t.includes(q))
  );
}

// Get presets by category
export function getPresetsByCategory(category) {
  return presets.filter(p => p.category === category);
}
