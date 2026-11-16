<script setup lang="ts">
/**
 * One setting: label and help on the left, the control in a right-aligned column, or below the
 * label when `stacked` (wide controls such as chips). Passing `error` — even `null` — reserves
 * the message line, so a message appearing later does not move anything below the row.
 */
defineProps<{ label: string; help?: string; error?: string | null; stacked?: boolean }>();
</script>

<template>
  <div :class="error === undefined ? 'py-2' : 'pt-2 pb-1'">
    <div
      class="flex min-h-12 gap-x-3 gap-y-2"
      :class="stacked ? 'flex-col justify-center' : 'items-center justify-between'"
    >
      <div class="min-w-0">
        <div class="text-[15px] leading-tight text-gray-100">{{ label }}</div>
        <div v-if="help" class="mt-0.5 text-xs leading-snug text-gray-400">{{ help }}</div>
      </div>
      <!-- One fixed width for every inline control keeps their edges aligned down the card. -->
      <div class="flex shrink-0" :class="stacked ? 'w-full' : 'w-38'">
        <slot />
      </div>
    </div>
    <p
      v-if="error !== undefined"
      class="h-4 truncate text-xs leading-4 text-rose-300"
      :title="error ?? undefined"
      aria-live="polite"
    >
      {{ error }}
    </p>
  </div>
</template>
