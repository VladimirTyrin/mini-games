<script setup lang="ts">
import { computed } from "vue";

/**
 * A few rows of a Numbers Match board for the help: numbers, crossed-out cells (`0`) and empty
 * ones (`null`), with the two cells of a pair marked as allowed or not.
 */
const props = defineProps<{
  cells: readonly (number | null)[];
  pair: readonly [number, number];
  allowed: boolean;
  caption: string;
}>();

const COLUMNS = 9;
const rows = computed(() => Math.ceil(props.cells.length / COLUMNS));

function cellClass(index: number, value: number | null): string {
  if (props.pair.includes(index)) return props.allowed ? "pair-ok" : "pair-no";
  if (value === null) return "empty";
  return value === 0 ? "crossed" : "plain";
}
</script>

<template>
  <figure class="flex flex-col gap-1.5">
    <div class="example-grid" :style="{ gridTemplateRows: `repeat(${rows}, auto)` }">
      <span v-for="(value, index) in cells" :key="index" class="cell" :class="cellClass(index, value)">
        {{ value === null ? "" : value === 0 ? "·" : value }}
      </span>
    </div>
    <figcaption class="flex items-start gap-1.5 text-sm leading-snug">
      <span :class="allowed ? 'text-emerald-400' : 'text-rose-400'" aria-hidden="true">{{
        allowed ? "✓" : "✗"
      }}</span>
      <span class="text-gray-200">{{ caption }}</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.example-grid {
  display: grid;
  grid-template-columns: repeat(9, minmax(0, 1fr));
  gap: 3px;
  width: min(100%, 17rem);
}

.cell {
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
  border-radius: 4px;
  font-size: 0.8125rem;
  font-weight: 600;
  line-height: 1;
}

.plain {
  background: var(--color-panel-raised);
  color: #d1d5db;
}

.crossed {
  background: rgb(31 41 55 / 0.5);
  color: #4b5563;
}

.empty {
  background: transparent;
}

.pair-ok {
  background: #065f46;
  box-shadow: inset 0 0 0 2px #34d399;
  color: #fff;
}

.pair-no {
  background: #7f1d1d;
  box-shadow: inset 0 0 0 2px #fb7185;
  color: #fff;
}
</style>
