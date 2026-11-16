<script setup lang="ts">
defineProps<{ label: string; icon: string; count: string; disabled: boolean; highlighted?: boolean }>();
const emit = defineEmits<{ press: [] }>();
</script>

<template>
  <button
    class="relative flex h-14 w-14 items-center justify-center rounded-full bg-panel-raised text-2xl transition-transform active:scale-95"
    :class="{ 'animate-pulse ring-2 ring-amber-400': highlighted }"
    :title="label"
    :aria-label="label"
    :disabled="disabled"
    @click="emit('press')"
  >
    <span aria-hidden="true">{{ icon }}</span>
    <span
      class="absolute -top-1 -right-1 flex h-6 min-w-6 items-center justify-center rounded-full bg-emerald-600 px-1 text-xs font-semibold text-white"
    >
      {{ count }}
    </span>
  </button>
</template>
