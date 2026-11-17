<script setup lang="ts">
import { computed, ref } from "vue";

import type { MatchController } from "@/match/match-controller";
import { useToastStore } from "@/stores/toasts";

import SeaGrid from "./SeaGrid.vue";
import {
  type CellPoint,
  crowdedShips,
  type EditorShip,
  fromWire,
  placed,
  rotated,
  shipCells,
  SIDE,
  toWire,
} from "./fleet-editor";

const props = defineProps<{ controller: MatchController }>();
const toasts = useToastStore();

/** The game's own random fleet: the rules, not the page, know what a fleet is. */
function randomFleet(): EditorShip[] {
  const command = props.controller.defaultSecretCommand();
  if (command?.game.case !== "seaBattle") return [];
  const placement = command.game.value.command;
  return placement.case === "placeFleet" ? placement.value.ships.map(fromWire) : [];
}

const ships = ref<EditorShip[]>(randomFleet());
const selected = ref<number | null>(null);
const sending = ref(false);
const overlay = ref<HTMLElement | null>(null);

interface Drag {
  index: number;
  pointerId: number;
  /** Where on the ship it was grabbed, in cells from the bow. */
  grabX: number;
  grabY: number;
  hasMoved: boolean;
  /** The ship was picked before this touch: a tap turns it (a first tap only picks it). */
  wasPicked: boolean;
}
let drag: Drag | null = null;

const crowded = computed(() => crowdedShips(ships.value));
const placement = computed(() => ({
  game: {
    case: "seaBattle" as const,
    value: { command: { case: "placeFleet" as const, value: { ships: ships.value.map(toWire) } } },
  },
}));
const refusal = computed(() => props.controller.secretRefusal(placement.value));

function cellAt(event: PointerEvent): CellPoint | null {
  const box = overlay.value?.getBoundingClientRect();
  if (!box || box.width === 0) return null;
  const x = Math.floor(((event.clientX - box.left) / box.width) * SIDE);
  const y = Math.floor(((event.clientY - box.top) / box.height) * SIDE);
  return { x: Math.min(Math.max(x, 0), SIDE - 1), y: Math.min(Math.max(y, 0), SIDE - 1) };
}

function shipAt(cell: CellPoint): number {
  return ships.value.findIndex((ship) => shipCells(ship).some((own) => own.x === cell.x && own.y === cell.y));
}

function replace(index: number, ship: EditorShip): void {
  ships.value = ships.value.map((current, at) => (at === index ? ship : current));
}

function onPointerDown(event: PointerEvent): void {
  const cell = cellAt(event);
  if (!cell || sending.value) return;
  const index = shipAt(cell);
  const ship = ships.value[index];
  if (!ship) {
    // A tap on open water moves the picked ship there.
    const picked = selected.value === null ? undefined : ships.value[selected.value];
    if (picked && selected.value !== null) replace(selected.value, placed(picked, cell.x, cell.y));
    return;
  }
  drag = {
    index,
    pointerId: event.pointerId,
    grabX: cell.x - ship.x,
    grabY: cell.y - ship.y,
    hasMoved: false,
    wasPicked: selected.value === index,
  };
  selected.value = index;
  overlay.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent): void {
  if (!drag || event.pointerId !== drag.pointerId) return;
  const cell = cellAt(event);
  const ship = ships.value[drag.index];
  if (!cell || !ship) return;
  const moved = placed(ship, cell.x - drag.grabX, cell.y - drag.grabY);
  if (moved.x === ship.x && moved.y === ship.y) return;
  drag.hasMoved = true;
  replace(drag.index, moved);
}

function onPointerUp(event: PointerEvent): void {
  if (!drag || event.pointerId !== drag.pointerId) return;
  const ship = ships.value[drag.index];
  if (!drag.hasMoved && drag.wasPicked && ship) replace(drag.index, rotated(ship));
  drag = null;
}

/** The browser took the gesture over (a scroll, say): the ship stays where it got to. */
function onPointerCancel(event: PointerEvent): void {
  if (drag?.pointerId === event.pointerId) drag = null;
}

function shuffle(): void {
  ships.value = randomFleet();
  selected.value = null;
}

async function ready(): Promise<void> {
  if (refusal.value || sending.value) return;
  sending.value = true;
  try {
    // Stays sending once accepted: the editor goes away with the frame that says so.
    await props.controller.submitSecret(placement.value);
  } catch (error) {
    sending.value = false;
    toasts.failure(error);
  }
}
</script>

<template>
  <div class="flex w-full flex-col items-center gap-3">
    <div class="relative" :style="{ width: 'min(92vw, 58vh, 440px)' }">
      <SeaGrid label="Your fleet" :ships="ships" :crowded="crowded" :selected="selected" />
      <div
        ref="overlay"
        class="absolute inset-0 cursor-pointer touch-none"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerCancel"
      />
    </div>
    <p class="text-center text-sm text-gray-400">
      Drag a ship to move it; tap it to pick it, tap again to turn it
    </p>
    <p v-if="crowded.size > 0" class="text-center text-sm text-amber-300">
      Ships may not touch, not even at a corner
    </p>
    <div class="flex gap-3">
      <button
        type="button"
        class="rounded-lg border border-white/15 px-4 py-2 text-sm font-medium text-gray-200 hover:bg-white/5"
        :disabled="sending"
        @click="shuffle"
      >
        Random
      </button>
      <button
        type="button"
        class="rounded-lg bg-emerald-600 px-6 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-40"
        :disabled="refusal !== null || sending"
        @click="ready"
      >
        Ready
      </button>
    </div>
  </div>
</template>
