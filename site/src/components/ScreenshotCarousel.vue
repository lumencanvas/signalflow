<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const screenshots = [
  {
    src: '/screenshots/mainapp.png',
    alt: 'CLASP Bridge main interface',
    caption: 'Main interface with bridges, mappings, and real-time monitor'
  },
  {
    src: '/screenshots/add-server.png',
    alt: 'Add server dialog',
    caption: 'Support for CLASP, OSC, MQTT, WebSocket, HTTP, Art-Net, and DMX'
  },
  {
    src: '/screenshots/create-bridge.png',
    alt: 'Create bridge dialog',
    caption: 'Connect any protocol to any other with visual bridge configuration'
  },
  {
    src: '/screenshots/create-mapping.png',
    alt: 'Create mapping dialog',
    caption: 'Route signals with source, transform, and target configuration'
  },
  {
    src: '/screenshots/transform.png',
    alt: 'Transform options',
    caption: 'Built-in transforms: scale, invert, clamp, threshold, expressions, and more'
  }
]

const currentIndex = ref(0)
let autoplayInterval = null

function next() {
  currentIndex.value = (currentIndex.value + 1) % screenshots.length
}

function prev() {
  currentIndex.value = (currentIndex.value - 1 + screenshots.length) % screenshots.length
}

function goTo(index) {
  currentIndex.value = index
}

function startAutoplay() {
  autoplayInterval = setInterval(next, 5000)
}

function stopAutoplay() {
  if (autoplayInterval) {
    clearInterval(autoplayInterval)
    autoplayInterval = null
  }
}

onMounted(() => {
  startAutoplay()
})

onUnmounted(() => {
  stopAutoplay()
})
</script>

<template>
  <div class="carousel" @mouseenter="stopAutoplay" @mouseleave="startAutoplay">
    <div class="carousel-container">
      <button class="carousel-btn prev" @click="prev" aria-label="Previous screenshot">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
      </button>

      <div class="carousel-track">
        <div
          v-for="(shot, index) in screenshots"
          :key="shot.src"
          class="carousel-slide"
          :class="{ active: index === currentIndex }"
        >
          <img :src="shot.src" :alt="shot.alt" />
        </div>
      </div>

      <button class="carousel-btn next" @click="next" aria-label="Next screenshot">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </button>
    </div>

    <p class="carousel-caption">{{ screenshots[currentIndex].caption }}</p>

    <div class="carousel-dots">
      <button
        v-for="(shot, index) in screenshots"
        :key="index"
        class="dot"
        :class="{ active: index === currentIndex }"
        @click="goTo(index)"
        :aria-label="`Go to screenshot ${index + 1}`"
      />
    </div>
  </div>
</template>

<style scoped>
.carousel {
  max-width: 900px;
  margin: 0 auto 3rem;
}

.carousel-container {
  position: relative;
  display: flex;
  align-items: center;
  gap: 1rem;
}

.carousel-track {
  position: relative;
  flex: 1;
  aspect-ratio: 4/3;
  overflow: hidden;
  border: 2px solid var(--border);
  background: var(--bg-darker);
}

.carousel-slide {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  transition: opacity 0.4s ease;
}

.carousel-slide.active {
  opacity: 1;
}

.carousel-slide img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #1a1a1a;
}

.carousel-btn {
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-card);
  border: 2px solid var(--border);
  color: var(--text);
  cursor: pointer;
  transition: all 0.2s;
}

.carousel-btn:hover {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.carousel-caption {
  text-align: center;
  color: var(--text-muted);
  margin-top: 1rem;
  font-size: 0.9rem;
  min-height: 1.5em;
}

.carousel-dots {
  display: flex;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 1rem;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--border);
  border: none;
  cursor: pointer;
  transition: all 0.2s;
}

.dot:hover {
  background: var(--text-muted);
}

.dot.active {
  background: var(--accent);
  transform: scale(1.2);
}

@media (max-width: 768px) {
  .carousel-btn {
    width: 36px;
    height: 36px;
  }

  .carousel-btn svg {
    width: 18px;
    height: 18px;
  }
}
</style>
