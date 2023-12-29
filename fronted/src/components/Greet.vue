<script setup lang="ts">
import { ref } from "vue";


const greetMsg = ref("");
const name = ref("");

async function greet() {

}
</script>

<template>
  <form class="row" @submit.prevent="greet">
    <input id="greet-input" v-model="name" placeholder="Enter a name..." />
    <button type="submit">Greet</button>
  </form>

  <p>{{ greetMsg }}</p>
</template>
