<script setup lang="ts">
import { reactive } from "vue";

import type { WorkerAction } from "./worker-input-pacer";

const emit = defineEmits<{ press: [action: WorkerAction]; release: [action: WorkerAction] }>();

/** Pointers down on each button: several fingers may share one, the press ends with the last. */
const pointers = reactive<Record<WorkerAction, Set<number>>>({
  left: new Set(),
  right: new Set(),
  jump: new Set(),
});

function down(action: WorkerAction, event: PointerEvent): void {
  // Capture keeps the hold even if the thumb slides off the button. It throws for a pointer that is
  // already gone (lifted before the handler ran); the pointerup handler still ends the press then.
  try {
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  } catch {
    // Nothing to capture.
  }
  const held = pointers[action];
  const wasIdle = held.size === 0;
  held.add(event.pointerId);
  if (wasIdle) emit("press", action);
}

function up(action: WorkerAction, event: PointerEvent): void {
  const held = pointers[action];
  if (!held.delete(event.pointerId) || held.size > 0) return;
  emit("release", action);
}

const BUTTON_CLASS =
  "control-button flex items-center justify-center rounded-2xl border border-white/10 shadow-md transition-transform";
const buttonState = (action: WorkerAction) =>
  pointers[action].size > 0 ? "scale-95 bg-emerald-700 text-white" : "bg-panel-raised text-gray-100";

const BUTTONS: readonly { action: WorkerAction; label: string; symbol: string }[] = [
  { action: "left", label: "Walk left", symbol: "◀" },
  { action: "right", label: "Walk right", symbol: "▶" },
];
</script>

<template>
  <div class="flex select-none items-center justify-between gap-3 pb-1" @contextmenu.prevent>
    <div class="flex gap-2">
      <button
        v-for="button in BUTTONS"
        :key="button.action"
        type="button"
        :aria-label="button.label"
        :class="[BUTTON_CLASS, buttonState(button.action), 'size-18 text-2xl']"
        @pointerdown="down(button.action, $event)"
        @pointerup="up(button.action, $event)"
        @pointercancel="up(button.action, $event)"
        @lostpointercapture="up(button.action, $event)"
      >
        {{ button.symbol }}
      </button>
    </div>
    <button
      type="button"
      aria-label="Jump"
      :class="[
        BUTTON_CLASS,
        buttonState('jump'),
        'h-18 w-28 text-base font-semibold uppercase tracking-wide',
      ]"
      @pointerdown="down('jump', $event)"
      @pointerup="up('jump', $event)"
      @pointercancel="up('jump', $event)"
      @lostpointercapture="up('jump', $event)"
    >
      Jump
    </button>
  </div>
</template>

<style scoped>
.control-button {
  -webkit-touch-callout: none;
  touch-action: none;
}
</style>
