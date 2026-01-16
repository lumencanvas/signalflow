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
      <a @click="scrollTo('spec')">Full Spec</a>
      <a @click="scrollTo('api')">API &amp; SDKs</a>
    </div>
  </nav>
</template>
