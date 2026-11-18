<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

import { create, fromBinary } from "@bufbuild/protobuf";

import type { MatchController } from "@/match/match-controller";
import { MatchCatchUpSchema } from "@/proto/minigames/v2/match_pb";
import { ReplayFileSchema } from "@/proto/minigames/v2/replay_pb";
import {
  SeaBattleCellMark,
  SeaBattlePhase,
  type SeaBattleRevealedFleet,
} from "@/proto/minigames/v2/sea_battle_pb";
import { useToastStore } from "@/stores/toasts";
import { seatLabels } from "@/util/players";
import { Mirror } from "@/wasm/core";

import FleetEditor from "./FleetEditor.vue";
import SeaGrid from "./SeaGrid.vue";
import ShipPip from "./ShipPip.vue";
import { type CellPoint, type EditorShip, fromWire } from "./fleet-editor";

const props = defineProps<{ controller: MatchController }>();
// Computed: Play again keeps this board and hands it the next match's controller.
const header = computed(() => props.controller.source.header);
const names = computed(() => seatLabels(header.value));
const toasts = useToastStore();
const sending = ref(false);

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "seaBattle" ? game.value : null;
});
const mySlot = computed(() => props.controller.mySlot);
/** The sea drawn below: the player's own, or the first seat's for someone watching. */
const homeSlot = computed(() => mySlot.value ?? 0);
/** The sea drawn above, the one fired at. */
const targetSlot = computed(() => 1 - homeSlot.value);
const myTurn = computed(
  () => view.value?.turnSlot !== undefined && view.value.turnSlot === mySlot.value && !sending.value,
);
const needsFleet = computed(
  () =>
    view.value?.phase === SeaBattlePhase.PLACING &&
    mySlot.value !== null &&
    view.value.ready[mySlot.value] !== true,
);

const ownFleet = computed<EditorShip[] | null>(() => {
  const secret = props.controller.seatSecret.value?.game;
  return secret?.case === "seaBattle" ? secret.value.ships.map(fromWire) : null;
});

/** A replay shows both fleets from the start: they are revealed at the end of its log. */
const replayFleets = ref<readonly SeaBattleRevealedFleet[]>([]);
onMounted(async () => {
  if (props.controller.source.kind !== "replay") return;
  const bytes = await props.controller.source.replayFile();
  if (!bytes) return;
  const replay = fromBinary(ReplayFileSchema, bytes);
  const mirror = Mirror.forHeader(header.value);
  try {
    mirror.applyCatchUp(
      create(MatchCatchUpSchema, { header: header.value, frames: replay.frames, nextTick: replay.endTick }),
    );
    const final = mirror.view(null).game;
    if (final.case === "seaBattle") replayFleets.value = final.value.revealed;
  } catch (error) {
    console.error("could not read the fleets of the replay", error);
  } finally {
    mirror.free();
  }
});

function fleetOf(slot: number): EditorShip[] {
  const revealed = view.value?.revealed[slot]?.ships ?? [];
  if (revealed.length > 0) return revealed.map(fromWire);
  if (slot === mySlot.value && ownFleet.value) return ownFleet.value;
  return (replayFleets.value[slot]?.ships ?? []).map(fromWire);
}

/** The ships sunk on `slot`'s sea, as everyone sees them. */
function sunkOf(slot: number): EditorShip[] {
  return (view.value?.waters[slot]?.sunk ?? []).map(fromWire);
}

/** The latest shot, on the sea it hit. */
function lastShotOn(slot: number): CellPoint | null {
  const last = view.value?.lastShot;
  if (!last?.cell || last.bySlot === slot) return null;
  return { x: last.cell.x, y: last.cell.y };
}

function seaLabel(slot: number): string {
  if (mySlot.value === null) return names.value[slot] ?? `Seat ${slot + 1}`;
  return slot === mySlot.value ? "Your fleet" : `${names.value[slot] ?? "Opponent"}'s sea`;
}

/** Cells of the other sea a shot may go to: those still unknown, on the player's turn. */
const targets = computed(() => {
  const marks = view.value?.waters[targetSlot.value]?.marks ?? [];
  if (!myTurn.value) return new Set<number>();
  return new Set(marks.flatMap((mark, index) => (mark === SeaBattleCellMark.UNKNOWN ? [index] : [])));
});

const afloat = computed(() => view.value?.waters[targetSlot.value]?.afloat ?? []);

// The time to place a fleet online, from the header; the server places a random one after it.
const now = ref(Date.now());
// Only while the player places a fleet against the clock; the board need not re-render otherwise.
const clock = setInterval(() => {
  if (needsFleet.value && header.value.secretSetupMs > 0) now.value = Date.now();
}, 500);
onBeforeUnmount(() => clearInterval(clock));
const secondsLeft = computed(() => {
  if (header.value.secretSetupMs === 0) return null;
  const deadline = Number(header.value.startedAtUnixMs) + header.value.secretSetupMs;
  return Math.max(0, Math.ceil((deadline - now.value) / 1000));
});

const status = computed(() => {
  const current = view.value;
  if (!current) return "";
  const watching = mySlot.value === null;
  switch (current.phase) {
    case SeaBattlePhase.PLACING: {
      if (!needsFleet.value)
        return watching ? "Fleets are being placed…" : "Waiting for the opponent's fleet…";
      const left = secondsLeft.value;
      if (left === null) return "Place your fleet";
      if (left === 0) return "Placing a random fleet…";
      return `Place your fleet · ${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;
    }
    case SeaBattlePhase.BATTLE: {
      const turn = current.turnSlot;
      if (turn === undefined) return "…";
      if (watching) return `${names.value[turn] ?? "A player"} fires`;
      return turn === mySlot.value ? "Your shot" : "Opponent's shot";
    }
    default: {
      const winner = current.winnerSlot;
      if (winner === undefined) return "Draw";
      if (watching) return `${names.value[winner] ?? "A player"} wins`;
      if (current.phase === SeaBattlePhase.FORFEITED && winner === mySlot.value)
        return "Opponent left — you win";
      return winner === mySlot.value ? "You win!" : "You lose";
    }
  }
});

async function fire(cell: CellPoint): Promise<void> {
  if (!myTurn.value) return;
  sending.value = true;
  try {
    await props.controller.submit({
      game: { case: "seaBattle", value: { command: { case: "fire", value: { cell } } } },
    });
  } catch (error) {
    toasts.failure(error);
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <div v-if="view" class="flex w-full flex-col items-center gap-2">
    <p class="text-center text-lg" :class="myTurn ? 'text-emerald-300' : 'text-gray-300'">{{ status }}</p>
    <FleetEditor v-if="needsFleet" :controller="controller" :seconds-left="secondsLeft" />
    <!-- Stacked on a phone; side by side on a wide screen held sideways (a tablet, a desktop), where
         both seas grow with the screen's height. The rows above the seas have one height there, so
         the seas line up. -->
    <div
      v-else
      class="flex w-full flex-col items-center gap-3 sm:landscape:flex-row sm:landscape:items-start sm:landscape:justify-center"
    >
      <section
        class="flex w-[min(92vw,42dvh,560px)] flex-col gap-1 sm:landscape:w-[min(56vw,calc(100dvh-13rem),680px)]"
      >
        <!-- A long name pushes the ships afloat onto a line of their own rather than being cut. -->
        <div
          class="flex flex-wrap items-center justify-between gap-x-2 gap-y-1 text-sm text-gray-300 sm:landscape:min-h-7 sm:landscape:items-end"
        >
          <span class="[overflow-wrap:anywhere]">{{ seaLabel(targetSlot) }}</span>
          <span class="flex shrink-0 flex-wrap gap-1" aria-label="Ships afloat">
            <ShipPip
              v-for="(length, index) in afloat"
              :key="index"
              :length="length"
              look="solid"
              size="small"
            />
          </span>
        </div>
        <SeaGrid
          :label="seaLabel(targetSlot)"
          :marks="view.waters[targetSlot]?.marks ?? []"
          :ships="fleetOf(targetSlot)"
          :sunk-ships="sunkOf(targetSlot)"
          :last-shot="lastShotOn(targetSlot)"
          :tappable="targets"
          @tap="fire"
        />
      </section>
      <section
        class="flex w-[min(72vw,34dvh,420px)] flex-col gap-1 sm:landscape:w-[min(38vw,calc(100dvh-13rem),520px)]"
      >
        <span
          class="text-sm text-gray-300 [overflow-wrap:anywhere] sm:landscape:flex sm:landscape:min-h-7 sm:landscape:items-end"
          >{{ seaLabel(homeSlot) }}</span
        >
        <SeaGrid
          :label="seaLabel(homeSlot)"
          :marks="view.waters[homeSlot]?.marks ?? []"
          :ships="fleetOf(homeSlot)"
          :sunk-ships="sunkOf(homeSlot)"
          :last-shot="lastShotOn(homeSlot)"
        />
      </section>
    </div>
  </div>
</template>
