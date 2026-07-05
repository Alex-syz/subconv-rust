<template>
  <div class="option-stack">
    <div class="switch-row"><div><label for="proxy-rules">代理规则集</label><p>关闭后直接从源站获取规则文件。</p></div><label class="switch-control"><input id="proxy-rules" type="checkbox" :checked="proxyRules" @change="emitCheckbox('update:proxyRules', $event)" /><span aria-hidden="true"><i /></span><b>{{ proxyRules ? '开启' : '关闭' }}</b></label></div>
    <div class="switch-row"><div><label for="standby-enabled">备用节点</label><p>仅加入手动选择分组。</p></div><label class="switch-control"><input id="standby-enabled" type="checkbox" :checked="standbyEnabled" @change="emitCheckbox('update:standbyEnabled', $event)" /><span aria-hidden="true"><i /></span><b>{{ standbyEnabled ? '开启' : '关闭' }}</b></label></div>
    <div v-if="standbyEnabled" class="field-group field-group--nested"><label for="standby-input">备用节点来源</label><textarea id="standby-input" class="text-control" :value="standby" rows="4" spellcheck="false" placeholder="多个备用节点可换行或用 | 分隔" @input="emit('update:standby', ($event.target as HTMLTextAreaElement).value)" /></div>
    <div class="field-group interval-field"><div class="field-heading"><label for="interval-input">更新间隔</label><span>默认 1800 秒</span></div><div class="unit-input"><input id="interval-input" class="text-control" type="text" inputmode="numeric" :value="interval" placeholder="1800" :aria-invalid="Boolean(intervalError)" :aria-describedby="intervalError ? 'interval-error' : undefined" @input="emit('update:interval', ($event.target as HTMLInputElement).value)" /><span>秒</span></div><p v-if="intervalError" id="interval-error" class="field-error">{{ intervalError }}</p></div>
  </div>
</template>
<script setup lang="ts">
defineProps<{ proxyRules: boolean; standbyEnabled: boolean; standby: string; interval: string; intervalError: string }>()
const emit = defineEmits<{ 'update:proxyRules': [value: boolean]; 'update:standbyEnabled': [value: boolean]; 'update:standby': [value: string]; 'update:interval': [value: string] }>()
function emitCheckbox(event: 'update:proxyRules' | 'update:standbyEnabled', domEvent: Event) {
  const checked = (domEvent.target as HTMLInputElement).checked
  if (event === 'update:proxyRules') emit('update:proxyRules', checked)
  else emit('update:standbyEnabled', checked)
}
</script>
