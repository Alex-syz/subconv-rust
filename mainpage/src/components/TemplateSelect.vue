<template>
  <div class="field-group">
    <div class="field-heading"><label for="template-select">配置模板</label><span class="field-heading__state">{{ stateLabel }}</span></div>
    <div class="select-wrap"><select id="template-select" class="text-control" :value="modelValue ?? ''" :disabled="loading || Boolean(loadError)" :aria-invalid="Boolean(error || loadError)" :aria-describedby="error || loadError ? 'template-error' : undefined" @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"><option value="" disabled>{{ placeholder }}</option><option v-for="template in templates" :key="template" :value="template">{{ template }}</option></select><svg viewBox="0 0 20 20" aria-hidden="true"><path d="m6 8 4 4 4-4" /></svg></div>
    <p v-if="error || loadError" id="template-error" class="field-error">{{ error || loadError }}</p>
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{ modelValue: string | null; templates: string[]; loading: boolean; loadError: string; error: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const stateLabel = computed(() => props.loading ? '正在加载' : props.loadError ? '不可用' : '运行时配置')
const placeholder = computed(() => props.loading ? '正在加载模板…' : props.loadError ? '模板加载失败' : '请选择模板')
</script>
