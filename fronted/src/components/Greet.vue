<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";

const greetMsg = ref("");
listen("runtimeRestart", (_event) => {
  greetMsg.value = "重启成功";
});


async function greet() {
  await invoke("plugin:http-server|restart_engine");
}
</script>

<template>
  <form class="row" @submit.prevent="greet">
    <button type="submit">重启jsRuntime</button>
  </form>

  <p>{{ greetMsg }}</p>
</template>
