<script setup>
import { ref, onMounted } from 'vue'
import NavBar from './components/NavBar.vue'
import HeroSection from './components/HeroSection.vue'
import LayersSection from './components/LayersSection.vue'
import SpecSection from './components/SpecSection.vue'
import ApiSection from './components/ApiSection.vue'
import CapabilitiesSection from './components/CapabilitiesSection.vue'
import DownloadsSection from './components/DownloadsSection.vue'
import FooterSection from './components/FooterSection.vue'

const specRef = ref(null)

function scrollToSpec() {
  specRef.value?.$el?.scrollIntoView({ behavior: 'smooth' })
}
</script>

<template>
  <NavBar />
  <HeroSection @scroll-to-spec="scrollToSpec" />
  <LayersSection />
  <SpecSection ref="specRef" />
  <ApiSection />
  <CapabilitiesSection />
  <DownloadsSection />
  <FooterSection />
</template>
