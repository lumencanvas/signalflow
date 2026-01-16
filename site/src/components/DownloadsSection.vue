<script setup>
const downloads = [
  {
    platform: 'macOS',
    desc: 'Apple Silicon & Intel',
    links: [
      { label: 'Apple Silicon (.dmg)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge-arm64.dmg' },
      { label: 'Intel (.dmg)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge-x64.dmg' }
    ]
  },
  {
    platform: 'Windows',
    desc: 'Windows 10/11',
    links: [
      { label: 'Installer (.exe)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge-Setup.exe' },
      { label: 'Portable (.zip)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge-win.zip' }
    ]
  },
  {
    platform: 'Linux',
    desc: 'AppImage & Deb',
    links: [
      { label: 'AppImage', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge.AppImage' },
      { label: 'Debian (.deb)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/clasp-bridge.deb' }
    ]
  }
]

const features = [
  'Multi-protocol servers (OSC, MQTT, WebSocket, HTTP, Art-Net, DMX)',
  'Visual signal routing with transforms',
  'Learn mode for quick MIDI/OSC mapping',
  'Real-time signal monitor',
  'Automatic state persistence'
]
</script>

<template>
  <section class="section downloads" id="downloads">
    <h2>DESKTOP APP</h2>
    <p class="section-desc">
      The CLASP Bridge desktop app provides a visual interface for protocol bridging,
      signal mapping, and monitoring.
    </p>

    <div class="features">
      <div class="feature" v-for="feature in features" :key="feature">
        <svg class="check-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
          <polyline points="20 6 9 17 4 12"/>
        </svg>
        <span>{{ feature }}</span>
      </div>
    </div>

    <div class="download-grid">
      <div class="download-card" v-for="dl in downloads" :key="dl.platform">
        <div class="platform-icon">
          <svg v-if="dl.platform === 'macOS'" width="48" height="48" viewBox="0 0 24 24" fill="currentColor">
            <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z"/>
          </svg>
          <svg v-else-if="dl.platform === 'Windows'" width="48" height="48" viewBox="0 0 24 24" fill="currentColor">
            <path d="M3 12V6.75l6-1.32v6.48L3 12zm17-9v8.75l-10 .15V5.21L20 3zM3 13l6 .09v6.81l-6-1.15V13zm17 .25V22l-10-1.91V13.1l10 .15z"/>
          </svg>
          <svg v-else width="48" height="48" viewBox="0 0 256 256" fill="currentColor">
            <path d="M128 20c-22 0-40 28-40 63 0 15 4 29 11 40-20 8-35 25-35 45 0 28 29 50 64 50s64-22 64-50c0-20-15-37-35-45 7-11 11-25 11-40 0-35-18-63-40-63zm-20 60a8 8 0 1 1 0-16 8 8 0 0 1 0 16zm40 0a8 8 0 1 1 0-16 8 8 0 0 1 0 16zm-20 25c8 0 15-3 15-7s-7-7-15-7-15 3-15 7 7 7 15 7z"/>
          </svg>
        </div>
        <h3>{{ dl.platform }}</h3>
        <p class="platform-desc">{{ dl.desc }}</p>
        <div class="download-links">
          <a v-for="link in dl.links" :key="link.label" :href="link.url" class="download-link">
            <svg class="download-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/>
            </svg>
            {{ link.label }}
          </a>
        </div>
      </div>
    </div>

    <div class="cli-section">
      <svg class="terminal-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
      </svg>
      <h3>Command Line</h3>
      <p>Install the CLI tool via Cargo:</p>
      <code class="cli-install">cargo install clasp-cli</code>
    </div>
  </section>
</template>

<style scoped>
.downloads {
  background: var(--bg-darker);
}

.section-desc {
  max-width: 600px;
  margin: 0 auto 2rem;
  text-align: center;
  color: var(--text-muted);
}

.features {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 1rem;
  margin-bottom: 3rem;
}

.feature {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: var(--bg-card);
  border: 1px solid var(--border);
  font-size: 0.875rem;
}

.check-icon {
  color: var(--accent);
  flex-shrink: 0;
}

.download-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1.5rem;
  max-width: 900px;
  margin: 0 auto 3rem;
}

.download-card {
  padding: 2rem;
  background: var(--bg-card);
  border: 2px solid var(--border);
  text-align: center;
}

.platform-icon {
  margin-bottom: 1rem;
  color: var(--text-muted);
}

.download-card h3 {
  font-size: 1.25rem;
  margin-bottom: 0.5rem;
}

.platform-desc {
  color: var(--text-muted);
  font-size: 0.875rem;
  margin-bottom: 1.5rem;
}

.download-links {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.download-link {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--accent);
  color: white;
  text-decoration: none;
  font-weight: 600;
  font-size: 0.875rem;
  transition: background 0.2s;
}

.download-link:hover {
  background: var(--accent-hover);
}

.download-icon {
  flex-shrink: 0;
}

.cli-section {
  max-width: 500px;
  margin: 0 auto;
  padding: 2rem;
  background: var(--bg-card);
  border: 2px solid var(--border);
  text-align: center;
}

.terminal-icon {
  color: var(--accent);
  margin-bottom: 1rem;
}

.cli-section h3 {
  margin-bottom: 0.5rem;
}

.cli-section p {
  color: var(--text-muted);
  margin-bottom: 1rem;
}

.cli-install {
  display: block;
  padding: 1rem;
  background: var(--bg-darker);
  border: 1px solid var(--border);
  font-family: monospace;
  font-size: 1rem;
  user-select: all;
}
</style>
