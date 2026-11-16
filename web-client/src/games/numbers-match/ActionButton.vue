<script setup lang="ts">
defineProps<{
  /** Visible caption under the button. */
  caption: string;
  /** Tooltip and accessible name; may mention the keyboard shortcut. */
  label: string;
  icon: string;
  count: string;
  disabled: boolean;
  highlighted?: boolean;
}>();
const emit = defineEmits<{ press: [] }>();
</script>

<template>
  <button
    class="group flex flex-col items-center gap-1 transition-transform active:scale-95"
    :title="label"
    :aria-label="label"
    :disabled="disabled"
    @click="emit('press')"
  >
    <span
      class="relative flex h-14 w-14 items-center justify-center rounded-full bg-panel-raised text-2xl"
      :class="{ 'animate-pulse ring-2 ring-amber-400': highlighted }"
    >
      <span aria-hidden="true">{{ icon }}</span>
      <span
        class="absolute -top-1 -right-1 flex h-6 min-w-6 items-center justify-center rounded-full bg-emerald-600 px-1 text-xs font-semibold text-white"
      >
        {{ count }}
      </span>
    </span>
    <span class="text-xs font-medium text-gray-300" aria-hidden="true">{{ caption }}</span>
  </button>
</template>
