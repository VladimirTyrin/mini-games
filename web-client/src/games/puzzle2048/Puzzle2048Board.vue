<script setup lang="ts">
import { computed, ref } from "vue";

import { useGameKeys } from "@/input/use-game-keys";
import { useSwipe } from "@/input/use-swipe";
import type { MatchController } from "@/match/match-controller";
import { type Puzzle2048Direction, Puzzle2048Phase } from "@/proto/minigames/v2/puzzle2048_pb";
import { useToastStore } from "@/stores/toasts";

import { directionFromKey, directionFromSwipe } from "./move-direction";
import ScoreBox from "./ScoreBox.vue";
import TileBoard from "./TileBoard.vue";
import { useElementSize } from "./use-element-size";

const props = defineProps<{ controller: MatchController }>();
const toasts = useToastStore();
const boardArea = ref<HTMLElement | null>(null);
const boardAreaSize = useElementSize(boardArea);

const view = computed(() => {
  const game = props.controller.current.value?.view.game;
  return game?.case === "puzzle2048" ? game.value : null;
});
const canPlay = computed(
  () => props.controller.mySlot !== null && view.value?.phase === Puzzle2048Phase.PLAYING,
);

const status = computed(() => {
  switch (view.value?.phase) {
    case Puzzle2048Phase.WON:
      return { text: `You reached ${view.value.target}!`, tone: "text-amber-300" };
    case Puzzle2048Phase.LOST:
      return { text: "No moves left", tone: "text-rose-400" };
    case Puzzle2048Phase.ABANDONED:
      return { text: "Game abandoned", tone: "text-gray-400" };
    default:
      return null;
  }
});

const boardWidthPx = computed(() => {
  const current = view.value;
  if (!current || current.height === 0) return 0;
  const { width, height } = boardAreaSize.value;
  return Math.floor(Math.min(width, (height * current.width) / current.height));
});

// The move count of the view a submitted move was based on: input waits for the next view so a
// quick double swipe is not checked against stale movable directions.
let movesAtSubmit: number | null = null;

function move(direction: Puzzle2048Direction): void {
  const current = view.value;
  if (!current || !canPlay.value || movesAtSubmit === current.moves) return;
  if (!current.movableDirections.includes(direction)) return;
  movesAtSubmit = current.moves;
  props.controller
    .submit({ game: { case: "puzzle2048", value: { command: { case: "move", value: direction } } } })
    .catch((error: unknown) => {
      movesAtSubmit = null;
      toasts.error(error instanceof Error ? error.message : String(error));
    });
}

useGameKeys((key) => {
  const direction = directionFromKey(key);
  if (direction === null) return false;
  move(direction);
  return true;
});
useSwipe(boardArea, { swipe: (swipe) => move(directionFromSwipe(swipe)) });
</script>

<template>
  <div v-if="view" class="flex flex-col items-center gap-3">
    <div class="flex w-full max-w-md items-stretch justify-between gap-2">
      <ScoreBox label="Score" :value="view.score" bump />
      <ScoreBox label="Moves" :value="view.moves" />
      <ScoreBox label="Target" :value="view.target" />
    </div>
    <p class="h-7 text-lg font-bold" :class="status?.tone">{{ status?.text }}</p>
    <div ref="boardArea" class="board-area flex min-h-0 w-full flex-1 items-center justify-center">
      <TileBoard :view="view" :style="{ width: `${boardWidthPx}px` }" />
    </div>
  </div>
</template>

<style scoped>
.board-area {
  min-height: 12rem;
  user-select: none;
}
</style>
