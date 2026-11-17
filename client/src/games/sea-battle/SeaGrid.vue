<script setup lang="ts">
import { computed } from "vue";

import { SeaBattleCellMark } from "@/proto/minigames/v2/sea_battle_pb";

import ShipSprite from "./ShipSprite.vue";
import { type CellPoint, type EditorShip, SIDE } from "./fleet-editor";

const props = withDefaults(
  defineProps<{
    /** Row by row; empty draws open water. */
    marks?: readonly SeaBattleCellMark[];
    ships?: readonly EditorShip[];
    /** Sunk ships, drawn whole in red so their size shows. */
    sunkShips?: readonly EditorShip[];
    /** Indexes of the cells that take taps. */
    tappable?: ReadonlySet<number>;
    /** Indexes of the cells shown as free to use (where a ship may still go). */
    hints?: ReadonlySet<number>;
    lastShot?: CellPoint | null;
    label: string;
  }>(),
  {
    marks: () => [],
    ships: () => [],
    sunkShips: () => [],
    tappable: () => new Set<number>(),
    hints: () => new Set<number>(),
    lastShot: null,
  },
);
const emit = defineEmits<{ tap: [cell: CellPoint] }>();

const COLUMNS = "ABCDEFGHIJ";

const cells = computed(() =>
  Array.from({ length: SIDE * SIDE }, (_, index) => ({
    index,
    x: index % SIDE,
    y: Math.floor(index / SIDE),
    mark: props.marks[index] ?? SeaBattleCellMark.UNKNOWN,
  })),
);

// Ships are drawn under the cells, so a shot's mark stays visible on a ship; open water is
// see-through.
function cellClass(cell: { index: number; x: number; y: number; mark: SeaBattleCellMark }): string {
  const isLast = props.lastShot?.x === cell.x && props.lastShot?.y === cell.y;
  // Whole class names only: Tailwind finds classes in the source text, not in built strings.
  const ring = isLast ? " ring-2 ring-inset ring-amber-300" : "";
  switch (cell.mark) {
    case SeaBattleCellMark.HIT:
      return "bg-orange-600/60" + ring;
    // The sunk ship is drawn underneath; the cell only marks the hit.
    case SeaBattleCellMark.SUNK:
      return ring.trim();
    case SeaBattleCellMark.MISS:
      return "bg-sky-950/60" + ring;
    case SeaBattleCellMark.CLEARED:
      return "bg-sky-950/40";
    default:
      if (props.hints.has(cell.index)) return "bg-sky-500/40 hover:bg-sky-400/60";
      return props.tappable.has(cell.index) ? "hover:bg-sky-400/40 active:bg-sky-300/50" : "";
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
</script>

<template>
  <div
    class="relative aspect-square w-full overflow-hidden rounded-md bg-sky-800 select-none"
    role="grid"
    :aria-label="label"
  >
    <div class="pointer-events-none absolute inset-0">
      <div
        v-for="(ship, index) in [...ships, ...sunkShips]"
        :key="index"
        class="absolute"
        :style="shipStyle(ship)"
      >
        <ShipSprite
          :length="ship.length"
          :horizontal="ship.horizontal"
          :look="index < ships.length ? 'afloat' : 'sunk'"
        />
      </div>
    </div>
    <div class="absolute inset-0 grid grid-cols-10 grid-rows-10">
      <button
        v-for="cell in cells"
        :key="cell.index"
        type="button"
        class="relative flex items-center justify-center border-[0.5px] border-sky-950/50 disabled:cursor-default disabled:opacity-100"
        :class="cellClass(cell)"
        :disabled="!tappable.has(cell.index)"
        :aria-label="`${COLUMNS[cell.x]}${cell.y + 1}`"
        @click="emit('tap', { x: cell.x, y: cell.y })"
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
