<script setup lang="ts">
import { computed, ref } from "vue";

import type { MatchController } from "@/match/match-controller";
import { TicTacToeEndReason, TicTacToeMark } from "@/proto/minigames/v2/tictactoe_pb";
import { useToastStore } from "@/stores/toasts";

import MarkIcon from "./MarkIcon.vue";

const props = defineProps<{ controller: MatchController }>();
const toasts = useToastStore();
const sending = ref(false);

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "tictactoe" ? game.value : null;
});
const mySlot = computed(() => props.controller.mySlot);
const myTurn = computed(
  () => view.value?.currentSlot !== undefined && view.value.currentSlot === mySlot.value,
);
const myMark = computed(() => {
  if (!view.value || mySlot.value === null) return null;
  if (view.value.xSlot === mySlot.value) return TicTacToeMark.X;
  if (view.value.oSlot === mySlot.value) return TicTacToeMark.O;
  return null;
});

const winningCells = computed(() => {
  const line = view.value?.end?.winningLine;
  if (!line?.start || !line.end) return new Set<number>();
  const width = view.value?.fieldWidth ?? 0;
  const cells = new Set<number>();
  const dx = Math.sign(line.end.x - line.start.x);
  const dy = Math.sign(line.end.y - line.start.y);
  for (let x = line.start.x, y = line.start.y; ; x += dx, y += dy) {
    cells.add(y * width + x);
    if (x === line.end.x && y === line.end.y) break;
  }
  return cells;
});

const status = computed(() => {
  const current = view.value;
  if (!current) return "";
  if (current.end) {
    if (current.end.reason === TicTacToeEndReason.DRAW) return "Draw";
    const won = current.end.winnerSlot === mySlot.value;
    if (current.end.reason === TicTacToeEndReason.PLAYER_DISCONNECTED)
      return won ? "Opponent left — you win" : "Game over";
    return mySlot.value === null ? "Game over" : won ? "You win!" : "You lose";
  }
  if (current.currentSlot === undefined) return "Choosing who starts…";
  return myTurn.value ? "Your move" : "Opponent's move";
});

async function place(index: number): Promise<void> {
  const current = view.value;
  if (!current || !myTurn.value || sending.value || current.cells[index] !== TicTacToeMark.UNSPECIFIED)
    return;
  sending.value = true;
  try {
    await props.controller.submit({
      game: {
        case: "tictactoe",
        value: {
          command: {
            case: "place",
            value: { x: index % current.fieldWidth, y: Math.floor(index / current.fieldWidth) },
          },
        },
      },
    });
  } catch (error) {
    toasts.failure(error);
  } finally {
    sending.value = false;
  }
}
</script>

<template>
  <div v-if="view" class="flex flex-col items-center gap-3">
    <p class="text-lg" :class="myTurn ? 'text-emerald-300' : 'text-gray-300'">
      {{ status }}
      <span v-if="myMark !== null" class="inline-flex items-center gap-1 text-sm text-gray-400">
        (you play
        <MarkIcon
          :mark="myMark"
          class="h-[1em] w-[1em]"
          :class="myMark === TicTacToeMark.X ? 'text-sky-300' : 'text-rose-300'"
        />)
      </span>
    </p>
    <!-- The board's own aspect ratio sizes it and fixed tracks split it, so no cell content can
         stretch a row: older Safari resolved the mark's percentage height inside aspect-ratio
         cells as the SVG's default 150px and pulled every row apart after the first move. -->
    <div
      class="grid w-full gap-px rounded bg-gray-700 p-px"
      :style="{
        gridTemplateColumns: `repeat(${view.fieldWidth}, minmax(0, 1fr))`,
        gridTemplateRows: `repeat(${view.fieldHeight}, minmax(0, 1fr))`,
        aspectRatio: `${view.fieldWidth} / ${view.fieldHeight}`,
        maxWidth: `min(90vw, calc(70vh * ${view.fieldWidth} / ${view.fieldHeight}))`,
      }"
    >
      <button
        v-for="(mark, index) in view.cells"
        :key="index"
        class="relative min-h-0 min-w-0 overflow-hidden disabled:cursor-default disabled:opacity-100"
        :class="[
          winningCells.has(index) ? 'bg-emerald-700' : 'bg-panel',
          mark === TicTacToeMark.X ? 'text-sky-300' : 'text-rose-300',
          view.lastMove && view.lastMove.y * view.fieldWidth + view.lastMove.x === index
            ? 'ring-1 ring-amber-300 ring-inset'
            : '',
        ]"
        :disabled="!myTurn || mark !== TicTacToeMark.UNSPECIFIED"
        @click="place(index)"
      >
        <MarkIcon
          v-if="mark !== TicTacToeMark.UNSPECIFIED"
          :mark="mark"
          class="absolute left-[15%] top-[15%] h-[70%] w-[70%]"
        />
      </button>
    </div>
  </div>
</template>
