<script setup lang="ts">
import { computed } from "vue";

/**
 * A ship seen from above, drawn for its length: a boat with a cabin, a cutter with a mast, then
 * turrets and a bridge for the longer ones. The bow points right (down when vertical); a notch
 * marks each deck. Fills its box: one cell across, `length` cells along.
 */
const props = defineProps<{
  length: number;
  horizontal: boolean;
  /** `ghost`: a ship still to place, drawn as an outline. */
  look: "afloat" | "sunk" | "ghost";
}>();

const CELL = 100;

const PALETTES = {
  afloat: { hull: "#94a3b8", deck: "#cbd5e1", detail: "#475569", edge: "#e2e8f0" },
  sunk: { hull: "#991b1b", deck: "#b91c1c", detail: "#450a0a", edge: "#fca5a5" },
  ghost: { hull: "none", deck: "none", detail: "none", edge: "#94a3b8" },
} as const;

const palette = computed(() => PALETTES[props.look]);
const span = computed(() => props.length * CELL);
const viewBox = computed(() =>
  props.horizontal ? `0 0 ${span.value} ${CELL}` : `0 0 ${CELL} ${span.value}`,
);
// Drawn lying with the bow to the right; turned a quarter for a vertical ship.
const turn = computed(() => (props.horizontal ? undefined : `translate(${CELL} 0) rotate(90)`));

const hull = computed(() => {
  const bow = span.value - 4;
  return `M 14 24 H ${bow - 30} Q ${bow - 4} 26 ${bow} 50 Q ${bow - 4} 74 ${bow - 30} 76 H 14 Q 6 76 6 68 V 32 Q 6 24 14 24 Z`;
});
const deck = computed(() => {
  const bow = span.value - 14;
  return `M 18 33 H ${bow - 26} Q ${bow - 6} 35 ${bow} 50 Q ${bow - 6} 65 ${bow - 26} 67 H 18 Q 14 67 14 63 V 37 Q 14 33 18 33 Z`;
});
const notches = computed(() => Array.from({ length: props.length - 1 }, (_, at) => (at + 1) * CELL));

/** Where the turrets stand, by length (centres along the ship). */
const TURRETS: Record<number, number[]> = { 1: [], 2: [], 3: [55, 245], 4: [50, 120, 340] };
const turrets = computed(() => TURRETS[props.length] ?? []);
</script>

<template>
  <svg :viewBox="viewBox" class="h-full w-full overflow-visible" aria-hidden="true">
    <g :transform="turn">
      <path :d="hull" :fill="palette.hull" :stroke="palette.edge" stroke-width="3" />
      <path :d="deck" :fill="palette.deck" />
      <line
        v-for="x in notches"
        :key="x"
        :x1="x"
        :x2="x"
        y1="27"
        y2="73"
        :stroke="look === 'ghost' ? palette.edge : palette.detail"
        stroke-width="3"
        stroke-opacity="0.6"
      />
      <template v-if="look !== 'ghost'">
        <!-- A boat: one cabin. -->
        <rect v-if="length === 1" x="26" y="40" width="30" height="20" rx="5" :fill="palette.detail" />
        <!-- A cutter: a cabin and a mast. -->
        <template v-else-if="length === 2">
          <rect x="40" y="39" width="46" height="22" rx="5" :fill="palette.detail" />
          <circle cx="135" cy="50" r="7" :fill="palette.detail" />
        </template>
        <!-- Turrets fore and aft, a bridge amidships. -->
        <template v-else>
          <rect :x="span / 2 - 32" y="38" width="56" height="24" rx="4" :fill="palette.detail" />
          <g v-for="x in turrets" :key="x">
            <line
              :x1="x"
              :x2="x + 26"
              y1="50"
              y2="50"
              :stroke="palette.detail"
              stroke-width="5"
              stroke-linecap="round"
            />
            <circle :cx="x" cy="50" r="12" :fill="palette.detail" />
          </g>
        </template>
      </template>
    </g>
  </svg>
</template>
