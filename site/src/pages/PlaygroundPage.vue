<script setup>
import { ref } from 'vue'
import ConnectionPanel from '../components/playground/ConnectionPanel.vue'
import ExplorerTab from '../components/playground/ExplorerTab.vue'
import ChatTab from '../components/playground/ChatTab.vue'
import SensorsTab from '../components/playground/SensorsTab.vue'
import SecurityTab from '../components/playground/SecurityTab.vue'
import DiscoveryTab from '../components/playground/DiscoveryTab.vue'
import ConsolePanel from '../components/playground/ConsolePanel.vue'

const activeTab = ref('explorer')
const consoleOpen = ref(true)

const tabs = [
  { id: 'explorer', label: 'Explorer' },
  { id: 'chat', label: 'Chat' },
  { id: 'sensors', label: 'Sensors' },
  { id: 'security', label: 'Security' },
  { id: 'discovery', label: 'Discovery' },
]
</script>

<template>
  <div class="playground">
    <div class="playground-header">
      <h1>CLASP Playground</h1>
      <p class="subtitle">Interactive protocol explorer and testing environment</p>
    </div>

    <div class="playground-layout">
      <aside class="sidebar">
        <ConnectionPanel />
      </aside>

      <main class="main-content">
        <div class="tab-bar">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            :class="['tab-btn', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>

        <div class="tab-content">
          <ExplorerTab v-if="activeTab === 'explorer'" />
          <ChatTab v-if="activeTab === 'chat'" />
          <SensorsTab v-if="activeTab === 'sensors'" />
          <SecurityTab v-if="activeTab === 'security'" />
          <DiscoveryTab v-if="activeTab === 'discovery'" />
        </div>
      </main>
    </div>

    <div :class="['console-container', { open: consoleOpen }]">
      <button class="console-toggle" @click="consoleOpen = !consoleOpen">
        {{ consoleOpen ? 'Hide' : 'Show' }} Console
      </button>
      <ConsolePanel v-if="consoleOpen" />
    </div>
  </div>
</template>

<style scoped>
.playground {
  min-height: calc(100vh - 60px);
  display: flex;
  flex-direction: column;
}

.playground-header {
  padding: 2rem 6vw 1.5rem;
  border-bottom: 1px solid rgba(0,0,0,0.12);
}

.playground-header h1 {
  font-size: 1.8rem;
  letter-spacing: 0.2em;
  margin: 0 0 0.5rem;
}

.playground-header .subtitle {
  margin: 0;
  opacity: 0.6;
  letter-spacing: 0.05em;
}

.playground-layout {
  display: grid;
  grid-template-columns: 280px 1fr;
  flex: 1;
  min-height: 0;
}

.sidebar {
  border-right: 1px solid rgba(0,0,0,0.12);
  padding: 1.5rem;
  background: rgba(255,255,255,0.3);
}

.main-content {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.tab-bar {
  display: flex;
  gap: 0;
  border-bottom: 1px solid rgba(0,0,0,0.12);
  padding: 0 1.5rem;
  background: rgba(255,255,255,0.2);
}

.tab-btn {
  background: none;
  border: none;
  padding: 1rem 1.5rem;
  font-family: inherit;
  font-size: 0.85rem;
  letter-spacing: 0.15em;
  cursor: pointer;
  opacity: 0.5;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  transition: opacity 0.15s, border-color 0.15s;
}

.tab-btn:hover {
  opacity: 0.8;
}

.tab-btn.active {
  opacity: 1;
  border-bottom-color: var(--accent);
}

.tab-content {
  flex: 1;
  overflow: auto;
  padding: 1.5rem;
}

.console-container {
  border-top: 1px solid rgba(0,0,0,0.12);
  background: rgba(255,255,255,0.4);
}

.console-container.open {
  height: 250px;
}

.console-toggle {
  display: block;
  width: 100%;
  padding: 0.5rem;
  background: rgba(0,0,0,0.03);
  border: none;
  font-family: inherit;
  font-size: 0.75rem;
  letter-spacing: 0.15em;
  cursor: pointer;
  opacity: 0.6;
}

.console-toggle:hover {
  opacity: 1;
  background: rgba(0,0,0,0.05);
}

@media (max-width: 900px) {
  .playground-layout {
    grid-template-columns: 1fr;
  }

  .sidebar {
    border-right: none;
    border-bottom: 1px solid rgba(0,0,0,0.12);
  }

  .tab-bar {
    overflow-x: auto;
  }

  .tab-btn {
    white-space: nowrap;
    padding: 0.8rem 1rem;
  }
}
</style>
