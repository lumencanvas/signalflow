<script setup>
import { ref } from 'vue'

const props = defineProps({
  code: {
    type: String,
    required: true,
  },
  language: {
    type: String,
    default: 'javascript',
  },
})

const copied = ref(false)

async function copyCode() {
  try {
    await navigator.clipboard.writeText(props.code)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (e) {
    console.error('Failed to copy:', e)
  }
}
</script>

<template>
  <div class="code-snippet">
    <div class="snippet-header">
      <span class="lang">{{ language }}</span>
      <button class="copy-btn" @click="copyCode">
        {{ copied ? 'Copied!' : 'Copy' }}
      </button>
    </div>
    <pre><code>{{ code }}</code></pre>
  </div>
</template>

<style scoped>
.code-snippet {
  margin-top: 0.8rem;
  border: 1px solid rgba(0,0,0,0.1);
  background: rgba(255,255,255,0.6);
  font-size: 0.75rem;
}

.snippet-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.3rem 0.6rem;
  background: rgba(0,0,0,0.04);
  border-bottom: 1px solid rgba(0,0,0,0.08);
}

.lang {
  font-size: 0.65rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.5;
}

.copy-btn {
  padding: 0.2rem 0.5rem;
  font-size: 0.65rem;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.15);
  cursor: pointer;
  font-family: inherit;
  letter-spacing: 0.05em;
}

.copy-btn:hover {
  background: rgba(0,0,0,0.05);
}

pre {
  margin: 0;
  padding: 0.6rem;
  overflow-x: auto;
  line-height: 1.5;
}

code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
