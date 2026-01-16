<script setup>
function scrollTo(id) {
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' })
}
</script>

<template>
  <nav class="topnav">
    <a class="brand" @click="scrollTo('top')">CLASP</a>
    <div class="navlinks">
      <a @click="scrollTo('layers')">Layers</a>
      <a @click="scrollTo('spec')">Spec</a>
      <a @click="scrollTo('api')">API</a>
      <a @click="scrollTo('downloads')">Downloads</a>
      <a href="https://github.com/lumencanvas/clasp" target="_blank" class="github-link">GitHub</a>
    </div>
  </nav>
</template>
