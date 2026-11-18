<script setup lang="ts">
import type { MessageInitShape } from "@bufbuild/protobuf";
import { computed, ref, watch } from "vue";

import { useGameKeys } from "@/input/use-game-keys";
import { useSwipe } from "@/input/use-swipe";
import { seatLabels } from "@/util/players";
import type { MatchController, ViewSnapshot } from "@/match/match-controller";
import {
  type StackAttackCommandSchema,
  StackAttackDirection,
  type StackAttackView,
  StackAttackWorkerStatus,
} from "@/proto/minigames/v2/stack_attack_pb";
import { tickProgress } from "@/render/tick-progress";
import { useCanvas } from "@/render/use-canvas";

import StackAttackControls from "./StackAttackControls.vue";
import StackAttackInfo from "./StackAttackInfo.vue";
import { drawStackAttackScene } from "./stack-attack-renderer";
import { StackAttackTransition } from "./stack-attack-transition";
import { freeWalkTarget } from "./walk-target";
import { WorkerAnimator } from "./worker-animator";
import { type WorkerAction, WorkerInputPacer } from "./worker-input-pacer";

const props = defineProps<{ controller: MatchController }>();

const container = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);

function stackAttackViewOf(snapshot: ViewSnapshot | null): StackAttackView | null {
  const game = snapshot?.view.game;
  return game?.case === "stackAttack" ? game.value : null;
}

const view = computed(() => stackAttackViewOf(props.controller.current.value));
// Labels, not raw names: two players called the same are told apart ("Alex · 2").
const seatNames = computed(() => seatLabels(props.controller.source.header));
const mySlot = computed(() => props.controller.mySlot);

const myWorker = computed(() => {
  const slot = mySlot.value;
  return slot === null ? undefined : view.value?.workers[slot];
});
const canAct = computed(
  () => myWorker.value?.status === StackAttackWorkerStatus.ALIVE && view.value?.endReason === undefined,
);
const status = computed(() => {
  if (!view.value || view.value.endReason !== undefined) return "";
  if (mySlot.value === null) return "Watching";
  if (myWorker.value?.status === StackAttackWorkerStatus.CRUSHED) return "You were crushed — watching";
  return "";
});

type Command = MessageInitShape<typeof StackAttackCommandSchema>["command"];

function commandFor(action: WorkerAction): Command {
  if (action === "jump") return { case: "jump", value: {} };
  const direction = action === "left" ? StackAttackDirection.LEFT : StackAttackDirection.RIGHT;
  return { case: "move", value: { direction } };
}

const animator = new WorkerAnimator(props.controller.tickMs, props.controller.mySlot);
const pacer = new WorkerInputPacer((action) => {
  const current = view.value;
  const slot = mySlot.value;
  if (!canAct.value || !current || slot === null || props.controller.isHeld()) return false;
  if (action !== "jump") {
    animator.faceOwn(action === "left" ? -1 : 1);
    const target = freeWalkTarget(current, slot, action);
    if (target !== null) animator.predictWalk(target, performance.now());
  }
  props.controller.submit({ game: { case: "stackAttack", value: { command: commandFor(action) } } }).then(
    () => pacer.acknowledged(),
    () => pacer.rejected(),
  );
  return true;
});

let transition: StackAttackTransition | null = null;

const { invalidate } = useCanvas(
  canvas,
  container,
  (context, size, now) => {
    if (!transition) {
      context.clearRect(0, 0, size.width, size.height);
      return;
    }
    drawStackAttackScene(context, size, {
      transition,
      animator,
      progress: tickProgress(props.controller.current.value, props.controller.tickMs, now),
      now,
    });
  },
  () => {
    if (!transition) return false;
    const now = performance.now();
    // While held (paused, counting down) nothing moves but effects: the next frame restarts it.
    return (
      (transition.view.endReason === undefined && !props.controller.isHeld()) ||
      tickProgress(props.controller.current.value, props.controller.tickMs, now) < 1 ||
      animator.hasEffectsAt(now)
    );
  },
);

watch(
  () => props.controller.current.value,
  (snapshot) => {
    const current = stackAttackViewOf(snapshot);
    if (!snapshot || !current) {
      transition = null;
      invalidate();
      return;
    }
    transition = new StackAttackTransition(
      current,
      stackAttackViewOf(props.controller.previous.value),
      snapshot.at,
    );
    pacer.frameArrived(myWorker.value?.queuedCommands ?? 0);
    if (!canAct.value) pacer.reset();
    animator.observe(transition, performance.now(), pacer.hasCommandsInFlight);
    pacer.pump();
    invalidate();
  },
  { immediate: true },
);

const KEY_ACTIONS = new Map<string, WorkerAction>([
  ["ArrowLeft", "left"],
  ["a", "left"],
  ["ArrowRight", "right"],
  ["d", "right"],
  ["ArrowUp", "jump"],
  ["w", "jump"],
  [" ", "jump"],
]);
const actionOfKey = (key: string) => KEY_ACTIONS.get(key.length === 1 ? key.toLowerCase() : key);

function release(action: WorkerAction): void {
  if (action !== "jump") pacer.release(action);
}

useGameKeys(
  (key) => {
    // Observers and replays: the keys belong to the replay bar.
    if (props.controller.mySlot === null) return false;
    const action = actionOfKey(key);
    if (!action) return false;
    pacer.press(action);
    return true;
  },
  {
    release: (key) => {
      const action = actionOfKey(key);
      if (action) release(action);
    },
    releaseAll: () => pacer.reset(),
  },
);
// Swipes stay as an alternative to the buttons: one swipe, one step (or a jump). Taps do nothing,
// so a short swipe can no longer jump by accident.
useSwipe(container, {
  swipe: (direction) => {
    if (direction === "up") pacer.press("jump");
    else if (direction === "left" || direction === "right") {
      pacer.press(direction);
      pacer.release(direction);
    }
  },
});
</script>

<template>
  <div
    class="flex min-h-0 flex-col gap-2 landscape:grid landscape:grid-cols-[minmax(0,1fr)_15rem] landscape:grid-rows-[minmax(0,1fr)_auto] landscape:gap-3 short-landscape:grid-cols-1"
  >
    <div ref="container" class="relative min-h-0 min-w-0 flex-1 select-none overflow-hidden">
      <canvas ref="canvas" class="absolute inset-0 block" />
    </div>
    <!-- contain:size keeps the info column from setting the row height: on a phone held sideways it
         would otherwise push the controls below the screen; it scrolls instead. -->
    <!-- A phone held sideways has no height to spare: the stats give way to the field there. -->
    <div
      class="shrink-0 portrait:order-first landscape:overflow-y-auto landscape:[contain:size] short-landscape:hidden"
    >
      <StackAttackInfo v-if="view" :view="view" :seat-names="seatNames" :status="status" />
    </div>
    <StackAttackControls
      v-if="mySlot !== null"
      class="hidden shrink-0 max-md:flex pointer-coarse:flex landscape:col-span-2"
      @press="pacer.press($event)"
      @release="release"
    />
  </div>
</template>
