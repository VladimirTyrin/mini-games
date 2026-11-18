<script setup lang="ts">
defineEmits<{ open: [] }>();
</script>

<template>
  <button
    type="button"
    class="flex size-8 shrink-0 items-center justify-center rounded-full border border-white/15 text-sm font-semibold text-gray-300 hover:bg-panel-raised hover:text-white"
    aria-label="How to play"
    title="How to play"
    @click="$emit('open')"
  >
    ?
  </button>
</template>
