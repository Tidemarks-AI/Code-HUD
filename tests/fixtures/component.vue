<script setup lang="ts">
import { ref } from 'vue'

const count = ref(0)

class Counter {
  value: number = 0

  increment() {
    this.value++
  }

  decrement() {
    this.value--
  }
}

interface Config {
  theme: string
  debug: boolean
}

function reset() {
  count.value = 0
}
</script>

<template>
  <button @click="reset">{{ count }}</button>
</template>
