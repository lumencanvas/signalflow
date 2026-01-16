<script setup>
import { ref, onMounted, watch } from 'vue'
import hljs from 'highlight.js/lib/core'
import javascript from 'highlight.js/lib/languages/javascript'
import python from 'highlight.js/lib/languages/python'
import rust from 'highlight.js/lib/languages/rust'
import c from 'highlight.js/lib/languages/c'
import json from 'highlight.js/lib/languages/json'
import plaintext from 'highlight.js/lib/languages/plaintext'
import 'highlight.js/styles/github.css'

hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('python', python)
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('c', c)
hljs.registerLanguage('json', json)
hljs.registerLanguage('plaintext', plaintext)

const props = defineProps({
  code: String,
  language: {
    type: String,
    default: 'plaintext'
  }
})

const codeRef = ref(null)

function highlight() {
  if (codeRef.value) {
    codeRef.value.textContent = props.code
    hljs.highlightElement(codeRef.value)
  }
}

onMounted(highlight)
watch(() => props.code, highlight)
</script>

<template>
  <pre><code ref="codeRef" :class="`language-${language}`">{{ code }}</code></pre>
</template>
