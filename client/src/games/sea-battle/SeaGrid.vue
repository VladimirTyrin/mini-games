<script setup lang="ts">
import { computed } from "vue";

import { SeaBattleCellMark } from "@/proto/minigames/v2/sea_battle_pb";

import { type CellPoint, type EditorShip, SIDE } from "./fleet-editor";

const props = withDefaults(
  defineProps<{
    /** Row by row; empty draws open water. */
    marks?: readonly SeaBattleCellMark[];
    ships?: readonly EditorShip[];
    /** Indexes into `ships` drawn as misplaced. */
    crowded?: ReadonlySet<number>;
    selected?: number | null;
    lastShot?: CellPoint | null;
    /** Unknown cells take taps. */
    canFire?: boolean;
    label: string;
  }>(),
  {
    marks: () => [],
    ships: () => [],
    crowded: () => new Set<number>(),
    selected: null,
    lastShot: null,
    canFire: false,
  },
);
const emit = defineEmits<{ fire: [cell: CellPoint] }>();

const COLUMNS = "ABCDEFGHIJ";

const cells = computed(() =>
  Array.from({ length: SIDE * SIDE }, (_, index) => ({
    x: index % SIDE,
    y: Math.floor(index / SIDE),
    mark: props.marks[index] ?? SeaBattleCellMark.UNKNOWN,
  })),
);

function isOpen(mark: SeaBattleCellMark): boolean {
  return mark === SeaBattleCellMark.UNKNOWN;
}

// Ships are drawn under the cells, so a shot's mark stays visible on a ship; open water is
// see-through.
function cellClass(cell: { x: number; y: number; mark: SeaBattleCellMark }): string {
  const isLast = props.lastShot?.x === cell.x && props.lastShot?.y === cell.y;
  const ring = isLast ? " ring-2 ring-inset ring-amber-300" : "";
  switch (cell.mark) {
    case SeaBattleCellMark.HIT:
      return `bg-orange-600/90${ring}`;
    case SeaBattleCellMark.SUNK:
      return `bg-red-900/90${ring}`;
    case SeaBattleCellMark.MISS:
      return `bg-sky-950/60${ring}`;
    case SeaBattleCellMark.CLEARED:
      return "bg-sky-950/40";
    default:
      return props.canFire ? "hover:bg-sky-400/40 active:bg-sky-300/50" : "";
  }
}

function shipStyle(ship: EditorShip): Record<string, string> {
  const unit = 100 / SIDE;
  return {
    left: `${ship.x * unit}%`,
    top: `${ship.y * unit}%`,
    width: `${(ship.horizontal ? ship.length : 1) * unit}%`,
    height: `${(ship.horizontal ? 1 : ship.length) * unit}%`,
  };
}

function shipClass(index: number): string {
  if (props.crowded.has(index)) return "bg-red-500/80 border-red-200";
  if (props.selected === index) return "bg-amber-300/90 border-amber-100";
  return "bg-slate-300/80 border-slate-100/80";
}
</script>

<template>
  <div
    class="relative aspect-square w-full overflow-hidden rounded-md bg-sky-800 select-none"
    role="grid"
    :aria-label="label"
  >
    <div class="pointer-events-none absolute inset-0">
      <div
        v-for="(ship, index) in ships"
        :key="index"
        class="sea-ship absolute p-[1.2%]"
        :style="shipStyle(ship)"
      >
        <div class="h-full w-full rounded-[30%] border" :class="shipClass(index)" />
      </div>
    </div>
    <div class="absolute inset-0 grid grid-cols-10 grid-rows-10">
      <button
        v-for="cell in cells"
        :key="cell.y * SIDE + cell.x"
        type="button"
        class="relative flex items-center justify-center border-[0.5px] border-sky-950/50 disabled:cursor-default disabled:opacity-100"
        :class="cellClass(cell)"
        :disabled="!canFire || !isOpen(cell.mark)"
        :aria-label="`${COLUMNS[cell.x]}${cell.y + 1}`"
        @click="emit('fire', { x: cell.x, y: cell.y })"
      >
        <span
          v-if="cell.mark === SeaBattleCellMark.MISS"
          class="h-[22%] w-[22%] rounded-full bg-sky-200/80"
        />
        <span
          v-else-if="cell.mark === SeaBattleCellMark.CLEARED"
          class="h-[14%] w-[14%] rounded-full bg-sky-200/40"
        />
        <span
          v-else-if="cell.mark === SeaBattleCellMark.HIT || cell.mark === SeaBattleCellMark.SUNK"
          class="text-[min(3.6vw,1rem)] leading-none font-bold text-white"
          >✕</span
        >
      </button>
    </div>
  </div>
</template>

<style scoped>
.sea-ship {
  transition:
    left 0.12s ease-out,
    top 0.12s ease-out;
}
</style>
