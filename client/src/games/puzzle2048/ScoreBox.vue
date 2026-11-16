<script setup lang="ts">
defineProps<{ label: string; value: number; bump?: boolean }>();
</script>

<template>
  <div class="min-w-0 flex-1 rounded-lg bg-[#574e44] px-2 py-1.5 text-center">
    <div class="text-[0.65rem] tracking-wider text-[#eee4da99] uppercase">{{ label }}</div>
    <!-- Keyed by value so the bump animation replays on every change. -->
    <div :key="bump ? value : undefined" class="text-xl font-bold text-white" :class="{ 'score-bump': bump }">
      {{ value }}
    </div>
  </div>
</template>

<style scoped>
.score-bump {
  animation: score-bump 250ms ease-out;
}

@keyframes score-bump {
  50% {
    transform: scale(1.15);
  }
}
</style>
