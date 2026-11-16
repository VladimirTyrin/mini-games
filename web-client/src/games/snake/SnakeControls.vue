<script setup lang="ts">
import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

const emit = defineEmits<{ turn: [direction: SnakeDirection] }>();

const BUTTON_CLASS =
  "flex size-16 items-center justify-center rounded-2xl border border-white/10 bg-panel-raised text-2xl text-gray-100 shadow-md active:scale-95 active:bg-emerald-700";

// Turning on pointerdown, not click: a tap must steer the very moment the finger lands.
function press(direction: SnakeDirection, event: PointerEvent): void {
  event.preventDefault();
  emit("turn", direction);
}
</script>

<template>
  <div
    class="grid select-none grid-cols-[repeat(3,4rem)] grid-rows-[repeat(2,4rem)] justify-center gap-2 pb-1"
    @contextmenu.prevent
  >
    <button
      type="button"
      aria-label="Up"
      :class="[BUTTON_CLASS, 'col-start-2']"
      @pointerdown="press(SnakeDirection.UP, $event)"
    >
      ▲
    </button>
    <button
      type="button"
      aria-label="Left"
      :class="[BUTTON_CLASS, 'col-start-1 row-start-2']"
      @pointerdown="press(SnakeDirection.LEFT, $event)"
    >
      ◀
    </button>
    <button
      type="button"
      aria-label="Down"
      :class="[BUTTON_CLASS, 'col-start-2 row-start-2']"
      @pointerdown="press(SnakeDirection.DOWN, $event)"
    >
      ▼
    </button>
    <button
      type="button"
      aria-label="Right"
      :class="[BUTTON_CLASS, 'col-start-3 row-start-2']"
      @pointerdown="press(SnakeDirection.RIGHT, $event)"
    >
      ▶
    </button>
  </div>
</template>
