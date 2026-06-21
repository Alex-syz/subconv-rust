<template>
  <div class="app-shell">
    <header class="site-header">
      <a class="brand" href="/" aria-label="SubConv RS 首页"><BrandMark /><span class="brand__copy"><strong>SubConv RS</strong><small>Rust-powered subscription converter</small></span></a>
      <a v-if="PROJECT_REPOSITORY_URL" class="repository-link" :href="PROJECT_REPOSITORY_URL" target="_blank" rel="noreferrer">GitHub <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M6 14 14 6m-6 0h6v6" /></svg></a>
    </header>
    <main class="console-grid">
      <section class="panel panel--controls" aria-labelledby="controls-heading"><span id="controls-heading" class="sr-only">订阅转换参数</span><slot name="controls" /></section>
      <section class="panel panel--result" aria-labelledby="result-heading"><span id="result-heading" class="sr-only">生成结果</span><slot name="result" /></section>
    </main>
    <footer class="site-footer"><span class="status-dot" aria-hidden="true" /><span>SubConv RS v3.0.0</span><span class="site-footer__divider" aria-hidden="true">/</span><span>Rust rewrite inspired by <a href="https://github.com/SubConv/SubConv" target="_blank" rel="noreferrer">SubConv</a></span></footer>
  </div>
</template>
<script setup lang="ts">
import BrandMark from './BrandMark.vue'
const PROJECT_REPOSITORY_URL = ''
</script>
