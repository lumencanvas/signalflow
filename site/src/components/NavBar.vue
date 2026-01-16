<script setup>
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

function scrollTo(id) {
  if (route.path !== '/') {
    router.push('/').then(() => {
      setTimeout(() => {
        document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' })
      }, 100)
    })
  } else {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' })
  }
}

function goHome() {
  if (route.path !== '/') {
    router.push('/')
  } else {
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }
}
</script>

<template>
  <nav class="topnav">
    <a class="brand" @click="goHome">CLASP</a>
    <div class="navlinks">
      <a @click="scrollTo('layers')">Layers</a>
      <a @click="scrollTo('spec')">Spec</a>
      <a @click="scrollTo('api')">API</a>
      <a @click="scrollTo('downloads')">Downloads</a>
      <router-link to="/playground" class="playground-link">Playground</router-link>
      <a href="https://github.com/lumencanvas/clasp" target="_blank" class="github-link">GitHub</a>
    </div>
  </nav>
</template>

<style scoped>
.playground-link {
  text-decoration: none;
  color: var(--ink);
  opacity: 0.75;
  letter-spacing: 0.12em;
  font-size: 0.8rem;
  cursor: pointer;
  padding: 0.2rem 0.6rem;
  border: 1px solid var(--accent);
  color: var(--accent);
  opacity: 1;
  transition: all 0.15s;
}

.playground-link:hover {
  background: var(--accent);
  color: var(--paper);
}

.playground-link.router-link-active {
  background: var(--accent);
  color: var(--paper);
}
</style>
