<template>
  <div class="field-group">
    <div class="field-heading"><label for="source-input">订阅来源</label><span>必填</span></div>
    <textarea id="source-input" class="text-control text-control--source" :value="modelValue" rows="6" spellcheck="false" placeholder="粘贴订阅链接或分享链接，多个来源可换行或用 | 分隔" :aria-invalid="Boolean(error)" :aria-describedby="error ? 'source-error source-hint' : 'source-hint'" @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)" />
    <p id="source-hint" class="field-hint">内容仅用于生成链接，不会在此页面上传。</p><p v-if="error" id="source-error" class="field-error">{{ error }}</p>
  </div>
</template>
<script setup lang="ts">
defineProps<{ modelValue: string; error: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>
