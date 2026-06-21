<template>
  <div class="result-panel">
    <div class="panel-heading"><p class="eyebrow">OUTPUT ENDPOINT</p><h2>生成结果</h2><p>确认参数后生成可直接导入客户端的新订阅链接。</p></div>
    <div class="result-display" :class="{ 'result-display--ready': result }"><div class="result-display__header"><span><i aria-hidden="true" />{{ result ? '链接已就绪' : '等待生成' }}</span><span>GET /sub</span></div><textarea class="result-output" :value="result" rows="8" readonly spellcheck="false" aria-label="生成的订阅链接" :placeholder="result ? '' : '生成后的订阅链接将显示在这里…'" /></div>
    <p v-if="configMessage" class="inline-notice inline-notice--error" role="alert">{{ configMessage }}</p><p v-if="copyError" class="inline-notice inline-notice--error" role="alert">{{ copyError }}</p><p class="sr-only" aria-live="polite">{{ copyState === 'copied' ? '链接已复制' : copyError }}</p>
    <div class="result-actions"><button class="button button--primary" type="button" :disabled="!canGenerate" @click="emit('generate')"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M4 10h12m-5-5 5 5-5 5" /></svg>生成链接</button><button class="button button--secondary" type="button" :disabled="!result" @click="emit('copy')"><svg viewBox="0 0 20 20" aria-hidden="true"><rect x="7" y="7" width="9" height="9" rx="2" /><path d="M13 7V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v5a2 2 0 0 0 2 2h1" /></svg>{{ copyState === 'copied' ? '已复制' : '复制链接' }}</button></div>
    <div class="privacy-note"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 2 4 5v4c0 4 2.5 7 6 9 3.5-2 6-5 6-9V5Z" /><path d="m7.5 10 1.5 1.5 3.5-4" /></svg><p><strong>本地构造</strong><br />此页面只拼接 URL；访问生成链接时，后端才会读取订阅内容。</p></div>
  </div>
</template>
<script setup lang="ts">
export type CopyState = 'idle' | 'copied'
defineProps<{ result: string; canGenerate: boolean; configMessage: string; copyState: CopyState; copyError: string }>()
const emit = defineEmits<{ generate: []; copy: [] }>()
</script>
