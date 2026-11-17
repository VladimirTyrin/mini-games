<script setup lang="ts">
/** A small ship for a list of ships: one square per deck, so its length can be counted. */
defineProps<{
  length: number;
  /** Solid: placed or afloat. Outlined: still to place. */
  look: "solid" | "outline";
  size: "small" | "medium";
}>();
</script>

<template>
  <span class="inline-flex gap-px" :aria-label="`${length}-deck ship`">
    <span
      v-for="deck in length"
      :key="deck"
      class="first:rounded-l-full last:rounded-r-full"
      :class="[
        size === 'medium' ? 'h-2 w-[11px]' : 'h-1.5 w-2.5',
        look === 'solid' ? 'bg-slate-300/85' : 'border border-slate-400/70',
      ]"
    />
  </span>
</template>
