<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";

import { NumbersMatchCellState, type NumbersMatchView } from "@/proto/minigames/v2/numbers_match_pb";

import { actionFrameKey, type CellMotion, cellMotions } from "./cell-motions";

const props = defineProps<{
  view: NumbersMatchView;
  selectedIndex: number | null;
  hintedIndices: ReadonlySet<number>;
  rejectedIndices: ReadonlySet<number>;
  rejectionSerial: number;
  interactive: boolean;
}>();
const emit = defineEmits<{ select: [index: number] }>();

const REFILL_STAGGER_MS = 12;
const MAX_REFILL_DELAY_MS = 300;

const grid = ref<HTMLElement | null>(null);
const frameKey = computed(() => actionFrameKey(props.view));
const motions = computed(() => cellMotions(props.view));

// Animated cells get a key per accepted command, so their animation restarts from the new view
// even mid-animation; still cells keep their element.
function cellKey(index: number): string {
  if (props.rejectedIndices.has(index)) return `rejected-${props.rejectionSerial}:${index}`;
  const motion = motions.value[index];
  const animated = motion && (motion.rowsShiftedUp > 0 || motion.justRemoved || motion.refillOrder !== null);
  return animated ? `${frameKey.value}:${index}` : String(index);
}

function cellClass(index: number, state: NumbersMatchCellState, motion: CellMotion | undefined) {
  return {
    active: state === NumbersMatchCellState.ACTIVE,
    removed: state === NumbersMatchCellState.REMOVED,
    empty: state === NumbersMatchCellState.EMPTY,
    selected: index === props.selectedIndex,
    hinted: props.hintedIndices.has(index) && index !== props.selectedIndex,
    rejected: props.rejectedIndices.has(index),
    shifted: (motion?.rowsShiftedUp ?? 0) > 0,
    "just-removed": motion?.justRemoved ?? false,
    refilled: motion !== undefined && motion.refillOrder !== null,
  };
}

// Each refill gets its own number colour, so new numbers are easy to tell from older ones.
const BATCH_CLASSES = ["", "batch-1", "batch-2", "batch-3", "batch-4"] as const;
function batchClass(batch: number): string {
  if (batch === 0) return "";
  return BATCH_CLASSES[((batch - 1) % (BATCH_CLASSES.length - 1)) + 1] ?? "";
}

function cellStyle(motion: CellMotion | undefined): Record<string, string> {
  return {
    "--rows-shifted-up": String(motion?.rowsShiftedUp ?? 0),
    "--refill-delay": `${Math.min((motion?.refillOrder ?? 0) * REFILL_STAGGER_MS, MAX_REFILL_DELAY_MS)}ms`,
  };
}

// Refilled values and hinted pairs can be outside the scrolled area of a tall board.
const cellToReveal = computed(() => {
  const action = props.view.lastAction?.action;
  if (action?.case === "refilled") return action.value.startIndex;
  const hint = props.view.hint?.hint;
  if (action?.case === "hintShown" && hint?.case === "pair") return hint.value.firstIndex;
  return null;
});
watch(frameKey, async () => {
  const index = cellToReveal.value;
  if (index === null) return;
  await nextTick();
  grid.value
    ?.querySelector(`[data-index="${index}"]`)
    ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
});
</script>

<template>
  <div
    ref="grid"
    class="cell-grid"
    :style="{
      '--columns': String(view.columns),
      gridTemplateColumns: `repeat(${view.columns}, minmax(0, 1fr))`,
    }"
  >
    <button
      v-for="(cell, index) in view.cells"
      :key="cellKey(index)"
      class="cell"
      :class="[cellClass(index, cell.state, motions[index]), batchClass(cell.batch)]"
      :style="cellStyle(motions[index])"
      :data-index="index"
      :disabled="!interactive || cell.state !== NumbersMatchCellState.ACTIVE"
      @click="emit('select', index)"
    >
      {{ cell.state === NumbersMatchCellState.EMPTY ? "" : cell.value }}
    </button>
  </div>
</template>

<style scoped>
.cell-grid {
  --gap: 4px;
  display: grid;
  width: min(100%, calc(var(--columns) * 3.5rem));
  gap: var(--gap);
  margin-inline: auto;
  container-type: inline-size;
}

.cell {
  aspect-ratio: 1;
  border-radius: 6px;
  font-size: calc(100cqw / var(--columns) * 0.5);
  font-weight: 600;
  line-height: 1;
  user-select: none;
}

.cell:disabled {
  opacity: 1;
  cursor: default;
}

.active {
  background: var(--color-panel-raised);
  color: #f3f4f6;
}

.active.batch-1 {
  color: #7dd3fc;
}

.active.batch-2 {
  color: #c4b5fd;
}

.active.batch-3 {
  color: #fcd34d;
}

.active.batch-4 {
  color: #f9a8d4;
}

.removed {
  background: rgb(31 41 55 / 0.5);
  color: #4b5563;
}

.empty {
  background: transparent;
}

.selected {
  background: #92400e;
  box-shadow: inset 0 0 0 2px #fbbf24;
  color: #fff;
}

.hinted {
  box-shadow: inset 0 0 0 2px #34d399;
  animation: hint-pulse 1s ease-in-out infinite;
}

.shifted {
  animation: cell-shift 150ms ease-out both;
}

.rejected {
  animation: cell-rejected 420ms ease-out;
}

@keyframes cell-rejected {
  0%,
  100% {
    transform: translateX(0);
  }
  15% {
    transform: translateX(-4px);
    background: #9f1239;
  }
  35% {
    transform: translateX(4px);
    background: #9f1239;
  }
  55% {
    transform: translateX(-3px);
  }
  75% {
    transform: translateX(2px);
  }
}

.just-removed {
  animation: cell-removed 220ms ease-out;
}

.shifted.just-removed {
  animation:
    cell-shift 150ms ease-out both,
    cell-removed-fade 220ms ease-out;
}

.refilled {
  animation: cell-refill 150ms ease-out var(--refill-delay) backwards;
}

@keyframes cell-shift {
  from {
    transform: translateY(calc(var(--rows-shifted-up) * (100% + var(--gap))));
  }
}

@keyframes cell-removed {
  from {
    transform: scale(1.15);
    background: #b45309;
    color: #fff;
  }
}

@keyframes cell-removed-fade {
  from {
    background: #b45309;
    color: #fff;
  }
}

@keyframes cell-refill {
  from {
    opacity: 0;
    transform: scale(0.3);
  }
}

@keyframes hint-pulse {
  50% {
    background: #065f46;
  }
}

@media (prefers-reduced-motion: reduce) {
  .cell {
    animation: none !important;
  }
}
</style>
