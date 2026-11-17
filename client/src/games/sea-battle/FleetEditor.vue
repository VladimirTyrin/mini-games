<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type { MatchController } from "@/match/match-controller";
import { useToastStore } from "@/stores/toasts";

import SeaGrid from "./SeaGrid.vue";
import {
  allowedCells,
  type CellPoint,
  cellIndex,
  cellsOf,
  type EditorShip,
  fromWire,
  roomForTheRest,
  shipsLeft,
  shipsOf,
  toWire,
} from "./fleet-editor";

const props = defineProps<{
  controller: MatchController;
  /** Online, the time left to place before the server places a random fleet; `null` without a limit. */
  secondsLeft: number | null;
}>();
const toasts = useToastStore();

/** The game's own random fleet: the rules, not the page, know what a fleet is. */
function randomFleet(): EditorShip[] {
  const command = props.controller.defaultSecretCommand();
  if (command?.game.case !== "seaBattle") return [];
  const placement = command.game.value.command;
  return placement.case === "placeFleet" ? placement.value.ships.map(fromWire) : [];
}

/** The ship lengths of a fleet, read from one the core made. */
const fleetLengths = randomFleet().map((ship) => ship.length);

/** The cells the player has filled, by index. */
const filled = ref<ReadonlySet<number>>(new Set());
const sending = ref(false);

const ships = computed(() => shipsOf(filled.value));
const allowed = computed(() => allowedCells(filled.value, fleetLengths));
const left = computed(() => shipsLeft(filled.value, fleetLengths));
const room = computed(() => roomForTheRest(filled.value, fleetLengths));
/** Nothing is lit while the fleet is on its way: no cell takes taps then. */
const hints = computed(() => (sending.value ? new Set<number>() : allowed.value));
const tappable = computed(() =>
  sending.value ? new Set<number>() : new Set([...filled.value, ...allowed.value]),
);
const placement = computed(() => ({
  game: {
    case: "seaBattle" as const,
    value: { command: { case: "placeFleet" as const, value: { ships: ships.value.map(toWire) } } },
  },
}));
const isComplete = computed(() => props.controller.secretRefusal(placement.value) === null);

/** A tap fills a free cell or clears a filled one, which shortens or splits its ship. */
function tap(cell: CellPoint): void {
  const index = cellIndex(cell);
  const next = new Set(filled.value);
  if (next.has(index)) next.delete(index);
  else if (allowed.value.has(index)) next.add(index);
  else return;
  filled.value = next;
}

function shuffle(): void {
  filled.value = cellsOf(randomFleet());
}

function clear(): void {
  filled.value = new Set();
}

/** Seconds before the limit at which a complete fleet on screen is sent for the player. */
const SEND_BEFORE_LIMIT_S = 2;
let isSentForPlayer = false;

// A fleet the player finished but did not send is theirs, not a random one. Sent once: by a clock
// that is off, the server may have placed a random fleet already, and saying so would only confuse.
watch([() => props.secondsLeft, isComplete], ([left, complete]) => {
  if (isSentForPlayer || left === null || left > SEND_BEFORE_LIMIT_S || !complete) return;
  isSentForPlayer = true;
  void send("quiet");
});

async function send(onFailure: "toast" | "quiet"): Promise<void> {
  if (!isComplete.value || sending.value) return;
  sending.value = true;
  try {
    // Stays sending once accepted: the editor goes away with the frame that says so.
    await props.controller.submitSecret(placement.value);
  } catch (error) {
    sending.value = false;
    if (onFailure === "toast") toasts.failure(error);
  }
}
</script>

<template>
  <div class="flex w-full flex-col items-center gap-3">
    <div :style="{ width: 'min(92vw, 54vh, 440px)' }">
      <SeaGrid label="Your fleet" :ships="ships" :tappable="tappable" :hints="hints" @tap="tap" />
    </div>
    <div class="flex flex-wrap justify-center gap-1.5" aria-label="Ships to place">
      <span
        v-for="(ship, index) in left"
        :key="index"
        class="h-2.5 rounded-sm"
        :class="ship.isPlaced ? 'bg-slate-300/80' : 'border border-slate-400/60'"
        :style="{ width: `${ship.length * 9}px` }"
      />
    </div>
    <p v-if="room === 'full'" class="text-center text-sm text-amber-300">
      No room left for the rest of the fleet: clear some decks
    </p>
    <p class="text-center text-sm text-gray-400">
      Tap a light cell to add a deck; tap a ship to take one away
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
        class="rounded-lg border border-white/15 px-4 py-2 text-sm font-medium text-gray-200 hover:bg-white/5"
        :disabled="sending || filled.size === 0"
        @click="clear"
      >
        Clear
      </button>
      <button
        type="button"
        class="rounded-lg bg-emerald-600 px-6 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-40"
        :disabled="!isComplete || sending"
        @click="send('toast')"
      >
        Ready
      </button>
    </div>
  </div>
</template>
