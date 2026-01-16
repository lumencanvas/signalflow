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
          <svg v-else width="48" height="48" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12.504 0c-.155 0-.311.003-.466.01a11.34 11.34 0 00-1.392.12 7.18 7.18 0 00-1.472.404 4.62 4.62 0 00-1.243.698 3.47 3.47 0 00-.892.978 3.51 3.51 0 00-.482 1.199 5.58 5.58 0 00-.12.875c-.013.251-.02.502-.02.754v.232c.001.253.009.505.025.756.024.377.072.752.145 1.122.07.36.17.713.296 1.057.137.37.32.72.545 1.041.127.18.27.348.424.504.217.22.466.402.74.546.38.2.792.333 1.22.395a6.07 6.07 0 001.476.036 4.76 4.76 0 001.347-.316c.436-.17.839-.41 1.194-.708a4.25 4.25 0 00.903-.995c.26-.396.466-.825.609-1.276.136-.428.22-.872.252-1.32.024-.335.034-.67.03-1.006l-.001-.09c-.005-.383-.035-.765-.089-1.144a6.69 6.69 0 00-.292-1.186 4.84 4.84 0 00-.547-1.082 3.78 3.78 0 00-.826-.866 3.87 3.87 0 00-1.062-.58 5.35 5.35 0 00-1.237-.286A10.45 10.45 0 0012.504 0zm-.179 1.433c.155 0 .31.004.464.013.279.016.556.048.83.096.233.04.462.098.685.172.192.064.377.147.553.246a2.1 2.1 0 01.449.338c.134.13.25.278.346.441.108.187.19.387.244.596.063.241.098.489.104.739.006.253-.011.506-.051.756a4.15 4.15 0 01-.213.82 3.04 3.04 0 01-.395.73 2.47 2.47 0 01-.583.56 2.76 2.76 0 01-.739.36 4.02 4.02 0 01-.855.167 5.32 5.32 0 01-.907-.002 3.56 3.56 0 01-.808-.15 2.35 2.35 0 01-.662-.34 2.05 2.05 0 01-.487-.52 2.7 2.7 0 01-.309-.664 4.36 4.36 0 01-.145-.74 6.08 6.08 0 01-.018-.738c.01-.232.035-.463.076-.691.045-.25.115-.495.209-.73a2.6 2.6 0 01.367-.629c.152-.195.334-.363.54-.5.236-.157.497-.277.774-.356.3-.086.612-.134.925-.143zm-5.2 9.178c-.062 0-.124.002-.186.006a3.97 3.97 0 00-.544.07c-.167.032-.33.08-.486.144-.133.055-.26.125-.377.208a1.2 1.2 0 00-.294.294 1.2 1.2 0 00-.163.395 2.08 2.08 0 00-.035.412c0 .146.012.291.036.434.024.143.06.284.107.42.05.15.117.292.199.424.093.15.21.283.346.392.186.15.405.258.642.315.263.064.536.078.805.041.242-.033.477-.108.694-.222.194-.102.37-.236.518-.397.12-.131.22-.28.294-.44.076-.166.127-.343.153-.524a2.7 2.7 0 00.011-.556 2.08 2.08 0 00-.11-.52 1.46 1.46 0 00-.256-.447 1.36 1.36 0 00-.39-.323 1.86 1.86 0 00-.498-.185 3.4 3.4 0 00-.566-.065zm10.05.003c-.062 0-.124.002-.186.006a3.97 3.97 0 00-.544.07c-.167.032-.33.08-.486.144-.133.055-.26.125-.377.208a1.2 1.2 0 00-.294.294 1.2 1.2 0 00-.163.395 2.08 2.08 0 00-.035.412c0 .146.012.291.036.434.024.143.06.284.107.42.05.15.117.292.199.424.093.15.21.283.346.392.186.15.405.258.642.315.263.064.536.078.805.041.242-.033.477-.108.694-.222.194-.102.37-.236.518-.397.12-.131.22-.28.294-.44.076-.166.127-.343.153-.524a2.7 2.7 0 00.011-.556 2.08 2.08 0 00-.11-.52 1.46 1.46 0 00-.256-.447 1.36 1.36 0 00-.39-.323 1.86 1.86 0 00-.498-.185 3.4 3.4 0 00-.566-.065zM6.51 17.193c-.237.003-.475.022-.71.057a4.63 4.63 0 00-.728.167 3.34 3.34 0 00-.673.293c-.204.115-.392.258-.557.425a2.18 2.18 0 00-.407.553 2.43 2.43 0 00-.226.676 3.73 3.73 0 00-.055.715c0 .244.019.487.057.727.04.256.107.505.2.744.1.256.232.498.392.72.18.249.4.465.654.64.287.198.61.341.95.42.383.09.779.102 1.167.037a3.62 3.62 0 001.067-.344c.308-.156.585-.364.818-.616a3 3 0 00.538-.837c.124-.299.203-.614.235-.935a4.47 4.47 0 00-.022-.908 3.3 3.3 0 00-.236-.832 2.64 2.64 0 00-.463-.699 2.5 2.5 0 00-.657-.509 3.14 3.14 0 00-.8-.296 4.73 4.73 0 00-.865-.086zm10.996 0c-.237.003-.475.022-.71.057a4.63 4.63 0 00-.728.167 3.34 3.34 0 00-.673.293c-.204.115-.392.258-.557.425a2.18 2.18 0 00-.407.553 2.43 2.43 0 00-.226.676 3.73 3.73 0 00-.055.715c0 .244.019.487.057.727.04.256.107.505.2.744.1.256.232.498.392.72.18.249.4.465.654.64.287.198.61.341.95.42.383.09.779.102 1.167.037a3.62 3.62 0 001.067-.344c.308-.156.585-.364.818-.616a3 3 0 00.538-.837c.124-.299.203-.614.235-.935a4.47 4.47 0 00-.022-.908 3.3 3.3 0 00-.236-.832 2.64 2.64 0 00-.463-.699 2.5 2.5 0 00-.657-.509 3.14 3.14 0 00-.8-.296 4.73 4.73 0 00-.865-.086z"/>
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
