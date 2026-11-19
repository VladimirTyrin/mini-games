<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from "vue";

import { t } from "@/i18n";
import { useGameKeys } from "@/input/use-game-keys";
import { type SwipeDirection, useSwipe } from "@/input/use-swipe";
import type { MatchController, ViewSnapshot } from "@/match/match-controller";
import { SnakeDirection, type SnakeView } from "@/proto/minigames/v2/snake_pb";
import { tickProgress } from "@/render/tick-progress";
import { useCanvas } from "@/render/use-canvas";
import { matchSeatNames } from "@/games/seat-names";

import SnakeControls from "./SnakeControls.vue";
import SnakeScoreboard from "./SnakeScoreboard.vue";
import { drawSnakeScene } from "./snake-renderer";
import { loadSnakeSpriteSheet, type SnakeSpriteSheet } from "./snake-sprite-sheet";

const props = defineProps<{ controller: MatchController }>();

const container = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);
const sprites = shallowRef<SnakeSpriteSheet | null>(null);

function snakeViewOf(snapshot: ViewSnapshot | null): SnakeView | null {
  const game = snapshot?.view.game;
  return game?.case === "snake" ? game.value : null;
}

const view = computed(() => snakeViewOf(props.controller.current.value));
const seatNames = computed(() => matchSeatNames(props.controller.source.header));

const mySeat = computed(() => {
  const slot = props.controller.mySlot;
  return slot === null ? undefined : view.value?.snakes[slot];
});
const canSteer = computed(() => mySeat.value?.alive === true && !view.value?.end);
const status = computed(() => {
  if (!view.value || view.value.end) return "";
  if (props.controller.mySlot === null) return t.games.snake.board.watching;
  return mySeat.value?.alive === false ? t.games.snake.board.outWatching : "";
});

const { invalidate } = useCanvas(
  canvas,
  container,
  (context, size, now) => {
    const snapshot = props.controller.current.value;
    const current = snakeViewOf(snapshot);
    if (!current) {
      context.clearRect(0, 0, size.width, size.height);
      return;
    }
    drawSnakeScene(context, size, {
      view: current,
      previous: snakeViewOf(props.controller.previous.value),
      progress: tickProgress(snapshot, props.controller.tickMs, now),
      sprites: sprites.value,
    });
  },
  () => tickProgress(props.controller.current.value, props.controller.tickMs, performance.now()) < 1,
);

watch(() => props.controller.current.value, invalidate);
onMounted(() => {
  void loadSnakeSpriteSheet().then(
    (sheet) => {
      sprites.value = sheet;
      invalidate();
    },
    // Without the sheet the board keeps drawing plain shapes.
    () => undefined,
  );
});

function turn(direction: SnakeDirection): void {
  if (!canSteer.value || props.controller.isHeld()) return;
  // Real-time input: a refused turn (full queue, reversing) is simply dropped.
  void props.controller
    .submit({ game: { case: "snake", value: { command: { case: "turn", value: { direction } } } } })
    .catch(() => undefined);
}

const KEY_DIRECTIONS = new Map<string, SnakeDirection>([
  ["ArrowUp", SnakeDirection.UP],
  ["ArrowDown", SnakeDirection.DOWN],
  ["ArrowLeft", SnakeDirection.LEFT],
  ["ArrowRight", SnakeDirection.RIGHT],
  ["w", SnakeDirection.UP],
  ["s", SnakeDirection.DOWN],
  ["a", SnakeDirection.LEFT],
  ["d", SnakeDirection.RIGHT],
]);

const SWIPE_DIRECTIONS: Record<SwipeDirection, SnakeDirection> = {
  up: SnakeDirection.UP,
  down: SnakeDirection.DOWN,
  left: SnakeDirection.LEFT,
  right: SnakeDirection.RIGHT,
};

useGameKeys((key) => {
  // Observers and replays: the keys belong to the replay bar.
  if (props.controller.mySlot === null) return false;
  const direction = KEY_DIRECTIONS.get(key.length === 1 ? key.toLowerCase() : key);
  if (direction === undefined) return false;
  turn(direction);
  return true;
});
useSwipe(container, { swipe: (direction) => turn(SWIPE_DIRECTIONS[direction]) });
</script>

<template>
  <!-- Portrait (phones, tablets held upright): the field takes the full width, the D-pad sits at the
       bottom edge. Landscape: a side column with the scores on top and the D-pad at the bottom, where
       the right thumb rests. Width breakpoints alone put an upright iPad into the side layout.
       contain:size stops a long scoreboard from stretching the row past the screen; it scrolls. -->
  <div class="flex min-h-0 flex-col gap-2 landscape:flex-row landscape:gap-3">
    <div ref="container" class="relative min-h-0 min-w-0 flex-1 select-none overflow-hidden">
      <canvas ref="canvas" class="absolute inset-0 block" />
    </div>
    <div class="flex shrink-0 flex-col gap-2 landscape:w-60 landscape:gap-3 landscape:[contain:size]">
      <SnakeScoreboard v-if="view" :view="view" :seat-names="seatNames" :status="status" />
      <SnakeControls
        v-if="controller.mySlot !== null"
        class="hidden max-md:grid pointer-coarse:grid landscape:mt-auto"
        @turn="turn"
      />
    </div>
  </div>
</template>
