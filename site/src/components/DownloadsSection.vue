<script setup>
import ScreenshotCarousel from './ScreenshotCarousel.vue'

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
      { label: 'Portable (.exe)', url: 'https://github.com/lumencanvas/clasp/releases/latest/download/CLASP.Bridge-portable.exe' }
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

    <ScreenshotCarousel />

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
            <path d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.132 1.884 1.071.771-.06 1.592-.536 2.257-1.306.631-.765 1.683-1.084 2.378-1.503.348-.199.629-.469.649-.853.023-.4-.2-.811-.714-1.376v-.097l-.003-.003c-.17-.2-.25-.535-.338-.926-.085-.401-.182-.786-.492-1.046h-.003c-.059-.054-.123-.067-.188-.135a.357.357 0 00-.19-.064c.431-1.278.264-2.55-.173-3.694-.533-1.41-1.465-2.638-2.175-3.483-.796-1.005-1.576-1.957-1.56-3.368.026-2.152.236-6.133-3.544-6.139zm.529 3.405h.013c.213 0 .396.062.584.198.19.135.33.332.438.533.105.259.158.459.166.724 0-.02.006-.04.006-.06v.105a.086.086 0 01-.004-.021l-.004-.024a1.807 1.807 0 01-.15.706.953.953 0 01-.213.335.71.71 0 00-.088-.042c-.104-.045-.198-.064-.284-.133a1.312 1.312 0 00-.22-.066c.05-.06.146-.133.183-.198.053-.128.082-.264.088-.402v-.02a1.21 1.21 0 00-.061-.4c-.045-.134-.101-.2-.183-.333-.084-.066-.167-.132-.267-.132h-.016c-.093 0-.176.03-.262.132a.8.8 0 00-.205.334 1.18 1.18 0 00-.09.4v.019c.002.089.008.179.02.267-.193-.067-.438-.135-.607-.202a1.635 1.635 0 01-.018-.2v-.02a1.772 1.772 0 01.15-.768c.082-.22.232-.406.43-.533a.985.985 0 01.594-.2zm-2.962.059h.036c.142 0 .27.048.399.135.146.129.264.288.344.465.09.199.14.4.153.667v.004c.007.134.006.2-.002.266v.08c-.03.007-.056.018-.083.024-.152.055-.274.135-.393.2.012-.09.013-.18.003-.267v-.015c-.012-.133-.04-.2-.082-.333a.613.613 0 00-.166-.267.248.248 0 00-.183-.064h-.021c-.071.006-.13.04-.186.132a.552.552 0 00-.12.27.944.944 0 00-.023.33v.015c.012.135.037.2.08.334.046.134.098.2.166.268.01.009.02.018.034.024-.07.057-.117.07-.176.136a.304.304 0 01-.131.068 2.62 2.62 0 01-.275-.402 1.772 1.772 0 01-.155-.667 1.759 1.759 0 01.08-.668 1.43 1.43 0 01.283-.535c.128-.133.26-.2.418-.2zm1.37 1.706c.332 0 .733.065 1.216.399.293.2.523.269 1.052.468h.003c.255.136.405.266.478.399v-.131a.571.571 0 01.016.47c-.123.31-.516.643-1.063.842v.002c-.268.135-.501.333-.775.465-.276.135-.588.292-1.012.267a1.139 1.139 0 01-.448-.067 3.566 3.566 0 01-.322-.198c-.195-.135-.363-.332-.612-.465v-.005h-.005c-.4-.246-.616-.512-.686-.71-.07-.268-.005-.47.193-.6.224-.135.38-.271.483-.336.104-.074.143-.102.176-.131h.002v-.003c.169-.202.436-.47.839-.601.139-.036.294-.065.466-.065zm2.8 2.142c.358 1.417 1.196 3.475 1.735 4.473.286.534.855 1.659 1.102 3.024.156-.005.33.018.513.064.646-1.671-.546-3.467-1.089-3.966-.22-.2-.232-.335-.123-.335.59.534 1.365 1.572 1.646 2.757.13.535.16 1.104.021 1.67.067.028.135.06.205.067 1.032.534 1.413.938 1.23 1.537v-.002c-.06-.135-.12-.2-.18-.264-.12-.135-.238-.2-.356-.2-.118-.068-.236-.068-.355-.2h-.066c-.117.066-.235.066-.353.135-.12.066-.238.135-.356.135-.075.003-.15.012-.223.024-.09.135-.222.27-.308.334-.158.135-.196.267-.126.465.012.063.03.126.06.131.03.136.09.2.12.268.03.066.06.135.06.2v.2c0 .268-.03.535-.09.736-.082.2-.174.333-.296.4-.12.066-.186.133-.246.133h-.12c-.09 0-.176-.03-.264-.06a.72.72 0 00-.12-.134c-.09-.066-.2-.132-.28-.2-.09-.066-.18-.135-.27-.135-.12 0-.24.002-.36.066-.12.065-.24.133-.33.2-.097.066-.174.135-.24.202-.063.066-.12.066-.18.066h-.18c-.06 0-.09-.03-.15-.066-.12-.066-.21-.2-.24-.333-.03-.066-.06-.203-.06-.333 0-.132 0-.265.03-.398.03-.135.06-.266.12-.4.18-.4.51-.734.9-1.002.152-.102.32-.2.478-.266.096-.045.187-.09.282-.132a.726.726 0 00-.12-.27c-.18-.267-.39-.333-.54-.4-.15-.066-.27-.135-.36-.2l-.12-.066c-.03-.066-.06-.07-.12-.135-.09-.065-.12-.135-.09-.265.03-.134.09-.2.18-.267.06 0 .12.002.18.068l.09.002c.03 0 .09-.002.15-.068.06-.066.12-.132.18-.132h.06c.09.066.21.2.3.334.09.133.18.266.21.398l.06.135.12.066c.12.002.21-.064.27-.2 0-.066.03-.135.03-.2.03-.066 0-.133 0-.2v-.134c0-.066-.03-.133-.06-.2l-.03-.066-.09-.135c-.03-.065-.06-.135-.09-.198-.03-.066-.06-.135-.09-.2v-.066c-.06-.2-.03-.333.03-.465.06-.135.12-.27.24-.4l.09-.066h.12c.21 0 .27.066.27.2v.066c-.03.066-.03.133-.03.2-.03.066 0 .135 0 .2 0 .066.03.135.06.2 0 .066.03.133.06.2l.06.066.06.135.03.066c.06.066.12.2.18.265l.09.068.06.066c.09.066.18.066.27.066.09-.002.15-.002.18-.068.06-.066.06-.135.09-.2l.03-.2c0-.135 0-.333-.03-.465l-.03-.2v-.135c0-.066.03-.066.03-.135.03-.066.06-.135.12-.198.059-.066.12-.135.21-.2a.96.96 0 01.18-.132c.06-.03.12-.066.18-.066h.27c.09 0 .15.066.21.133.03.065.06.135.09.198.03.066.03.2.03.268 0 .066-.03.2-.06.266l-.06.2c-.03.135-.06.2-.12.265-.06.135-.12.2-.21.333-.09.066-.18.2-.27.27l-.12.132-.12.066c-.06.068-.09.135-.09.2 0 .068.03.135.06.2v.002c.03.066.06.135.09.2v.065l.06.068.03.2c0 .066-.03.135-.06.2-.03.066-.09.135-.12.135-.09.066-.18.068-.3.068-.09 0-.21-.002-.3-.068-.09-.066-.21-.198-.27-.332l-.06-.135-.03-.2-.03-.066c0-.066-.03-.135-.03-.2-.03-.068-.03-.068-.06-.135 0-.066 0-.135.03-.2z"/>
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
