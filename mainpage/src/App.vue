<template>
  <AppShell>
    <template #controls>
      <form id="conversion-form" class="control-form" novalidate @submit.prevent="generateLink">
        <div class="panel-heading"><p class="eyebrow">INPUT PIPELINE</p><h1>转换参数</h1><p>输入订阅来源，并按需调整 Mihomo 配置选项。</p></div>
        <SourceInput :model-value="source" :error="errors.source" @update:model-value="updateSource" />
        <TemplateSelect :model-value="selectedTemplate" :templates="templates" :loading="configLoading" :load-error="configError" :error="errors.template" @update:model-value="updateTemplate" />
        <OptionControls :proxy-rules="proxyRules" :standby-enabled="standbyEnabled" :standby="standby" :interval="interval" :interval-error="errors.interval" @update:proxy-rules="proxyRules = $event" @update:standby-enabled="standbyEnabled = $event" @update:standby="standby = $event" @update:interval="updateInterval" />
      </form>
    </template>
    <template #result>
      <ResultPanel :result="result" :can-generate="!configLoading && !configError" :config-message="configError" :copy-state="copyState" :copy-error="copyError" @generate="generateLink" @copy="copyLink" />
    </template>
  </AppShell>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import AppShell from './components/AppShell.vue'
import OptionControls from './components/OptionControls.vue'
import ResultPanel, { type CopyState } from './components/ResultPanel.vue'
import SourceInput from './components/SourceInput.vue'
import TemplateSelect from './components/TemplateSelect.vue'
import { loadRuntimeConfig } from './lib/runtime-config'
import { buildSubscriptionUrl, type SubscriptionUrlField } from './lib/subscription-url'

const source = ref('')
const selectedTemplate = ref<string | null>(null)
const templates = ref<string[]>([])
const interval = ref('')
const proxyRules = ref(true)
const standbyEnabled = ref(false)
const standby = ref('')
const result = ref('')
const configLoading = ref(true)
const configError = ref('')
const copyState = ref<CopyState>('idle')
const copyError = ref('')
const errors = reactive<Record<SubscriptionUrlField, string>>({ source: '', template: '', interval: '' })
let copyResetTimer: number | undefined

onMounted(async () => {
  const runtimeConfig = await loadRuntimeConfig()
  configLoading.value = false
  if (runtimeConfig.ok) {
    templates.value = runtimeConfig.templates
    selectedTemplate.value = runtimeConfig.selected
  } else configError.value = runtimeConfig.message
})

onBeforeUnmount(() => {
  if (copyResetTimer !== undefined) window.clearTimeout(copyResetTimer)
})

function clearErrors() { errors.source = ''; errors.template = ''; errors.interval = '' }
function updateSource(value: string) { source.value = value; errors.source = '' }
function updateTemplate(value: string) { selectedTemplate.value = value; errors.template = '' }
function updateInterval(value: string) { interval.value = value; errors.interval = '' }

function generateLink() {
  clearErrors()
  copyError.value = ''
  const generated = buildSubscriptionUrl({ origin: window.location.origin, source: source.value, template: selectedTemplate.value, interval: interval.value, standbyEnabled: standbyEnabled.value, standby: standby.value, proxyRules: proxyRules.value })
  if (!generated.ok) {
    errors[generated.field] = generated.message
    result.value = ''
    return
  }
  result.value = generated.url
}

async function copyLink() {
  if (!result.value) return
  copyError.value = ''
  try {
    await navigator.clipboard.writeText(result.value)
    copyState.value = 'copied'
    if (copyResetTimer !== undefined) window.clearTimeout(copyResetTimer)
    copyResetTimer = window.setTimeout(() => { copyState.value = 'idle' }, 1600)
  } catch {
    copyState.value = 'idle'
    copyError.value = '自动复制失败，请手动选择上方链接复制。'
  }
}
</script>
