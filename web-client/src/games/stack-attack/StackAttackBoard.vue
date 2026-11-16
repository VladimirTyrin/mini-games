<script setup lang="ts">
import type { MessageInitShape } from "@bufbuild/protobuf";
import { computed, ref, watch } from "vue";

import { useGameKeys } from "@/input/use-game-keys";
import { useSwipe } from "@/input/use-swipe";
import type { MatchController, ViewSnapshot } from "@/match/match-controller";
import {
  type StackAttackCommandSchema,
  StackAttackDirection,
  type StackAttackView,
  StackAttackWorkerStatus,
} from "@/proto/minigames/v2/stack_attack_pb";
import { tickProgress } from "@/render/tick-progress";
import { useCanvas } from "@/render/use-canvas";

import StackAttackInfo from "./StackAttackInfo.vue";
import { drawStackAttackScene } from "./stack-attack-renderer";

const props = defineProps<{ controller: MatchController }>();

const container = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);

function stackAttackViewOf(snapshot: ViewSnapshot | null): StackAttackView | null {
  const game = snapshot?.view.game;
  return game?.case === "stackAttack" ? game.value : null;
}

const view = computed(() => stackAttackViewOf(props.controller.current.value));
const seatNames = computed(() =>
  props.controller.source.header.seats.map((seat) => seat.player?.displayName ?? `Player ${seat.slot + 1}`),
);

const myWorker = computed(() => {
  const slot = props.controller.mySlot;
  return slot === null ? undefined : view.value?.workers[slot];
});
const canAct = computed(
  () => myWorker.value?.status === StackAttackWorkerStatus.ALIVE && view.value?.endReason === undefined,
);
const status = computed(() => {
  if (!view.value || view.value.endReason !== undefined) return "";
  if (props.controller.mySlot === null) return "Watching";
  if (myWorker.value?.status === StackAttackWorkerStatus.CRUSHED) return "You were crushed — watching";
  return "";
});

const { invalidate } = useCanvas(
  canvas,
  container,
  (context, size, now) => {
    const snapshot = props.controller.current.value;
    const current = stackAttackViewOf(snapshot);
    if (!current) {
      context.clearRect(0, 0, size.width, size.height);
      return;
    }
    drawStackAttackScene(context, size, {
      view: current,
      previous: stackAttackViewOf(props.controller.previous.value),
      progress: tickProgress(snapshot, props.controller.tickMs, now),
    });
  },
  () => tickProgress(props.controller.current.value, props.controller.tickMs, performance.now()) < 1,
);
watch(() => props.controller.current.value, invalidate);

function send(command: MessageInitShape<typeof StackAttackCommandSchema>["command"]): void {
  if (!canAct.value) return;
  // Real-time input: a refused command (full queue) is simply dropped.
  void props.controller.submit({ game: { case: "stackAttack", value: { command } } }).catch(() => undefined);
}

const moveLeft = () => send({ case: "move", value: { direction: StackAttackDirection.LEFT } });
const moveRight = () => send({ case: "move", value: { direction: StackAttackDirection.RIGHT } });
const jump = () => send({ case: "jump", value: {} });

const KEY_ACTIONS = new Map<string, () => void>([
  ["ArrowLeft", moveLeft],
  ["a", moveLeft],
  ["ArrowRight", moveRight],
  ["d", moveRight],
  ["ArrowUp", jump],
  ["w", jump],
  [" ", jump],
]);

useGameKeys((key) => {
  const action = KEY_ACTIONS.get(key.length === 1 ? key.toLowerCase() : key);
  if (!action) return false;
  action();
  return true;
});
useSwipe(container, {
  swipe: (direction) => {
    if (direction === "left") moveLeft();
    else if (direction === "right") moveRight();
    else if (direction === "up") jump();
  },
  tap: jump,
});
</script>

<template>
  <div class="flex min-h-0 flex-col gap-2 md:flex-row md:gap-3">
    <div ref="container" class="relative min-h-0 min-w-0 flex-1 select-none overflow-hidden">
      <canvas ref="canvas" class="absolute inset-0 block" />
    </div>
    <StackAttackInfo
      v-if="view"
      :view="view"
      :seat-names="seatNames"
      :status="status"
      class="shrink-0 md:w-60 md:self-start"
    />
  </div>
</template>
