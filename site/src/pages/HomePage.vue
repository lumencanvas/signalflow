<script setup>
import { ref } from 'vue'
import HeroSection from '../components/HeroSection.vue'
import LayersSection from '../components/LayersSection.vue'
import SpecSection from '../components/SpecSection.vue'
import ApiSection from '../components/ApiSection.vue'
import CapabilitiesSection from '../components/CapabilitiesSection.vue'
import DownloadsSection from '../components/DownloadsSection.vue'
import FooterSection from '../components/FooterSection.vue'

const specRef = ref(null)

function scrollToSpec() {
  specRef.value?.$el?.scrollIntoView({ behavior: 'smooth' })
}
</script>

<template>
  <HeroSection @scroll-to-spec="scrollToSpec" />
  <LayersSection />
  <DownloadsSection />
  <ApiSection />
  <SpecSection ref="specRef" />
  <CapabilitiesSection />
  <FooterSection />
</template>
