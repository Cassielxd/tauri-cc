<template>
  <t-config-provider :global-config="getComponentsLocale">
    <router-view :key="locale" :class="[mode]" />
  </t-config-provider>
</template>
<script setup lang="ts">
import { computed } from 'vue';
import { useLocale } from '@/locales/useLocale';
import { useSettingStore } from '@/store';
import {invoke} from "@tauri-apps/api";
import {listen} from "@tauri-apps/api/event";
const store = useSettingStore();

const mode = computed(() => {
  return store.displayMode;
});
invoke('plugin:ipcs|send_to_deno', {key:"main", name: 'testIpc', content: {url:"http://localhost:8080/demo/test3333/hello",method:"POST",data:"aaaa"} }).then((res) => {
  console.log(res);
}).catch((err: any) => {
  console.log(err);
});
const unlisten =  listen('testIpc', (event) => {
  console.log(event);
})
</script>
<style lang="less" scoped>
#nprogress .bar {
  background: var(--td-brand-color) !important;
}
</style>
