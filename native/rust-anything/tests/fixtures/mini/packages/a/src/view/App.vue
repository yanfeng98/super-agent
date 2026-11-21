<template>
  <div>{{ label }}</div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { count } from '../util'

const label = ref(String(count('a b')))
</script>
