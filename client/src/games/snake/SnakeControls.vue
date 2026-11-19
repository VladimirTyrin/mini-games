<script setup lang="ts">
import { t } from "@/i18n";
import { SnakeDirection } from "@/proto/minigames/v2/snake_pb";

// Pictures, not words: the same in every language.
const ARROWS = { up: "▲", left: "◀", down: "▼", right: "▶" } as const;

const emit = defineEmits<{ turn: [direction: SnakeDirection] }>();

const BUTTON_CLASS =
  "flex size-16 md:size-18 items-center justify-center rounded-2xl border border-white/10 bg-panel-raised text-2xl text-gray-100 shadow-md active:scale-95 active:bg-emerald-700";

// Turning on pointerdown, not click: a tap must steer the very moment the finger lands.
function press(direction: SnakeDirection, event: PointerEvent): void {
  event.preventDefault();
  emit("turn", direction);
}
</script>

<template>
  <div
    class="grid select-none grid-cols-[repeat(3,4rem)] grid-rows-[repeat(2,4rem)] md:grid-cols-[repeat(3,4.5rem)] md:grid-rows-[repeat(2,4.5rem)] justify-center gap-2 pb-1"
    @contextmenu.prevent
  >
    <button
      type="button"
      :aria-label="t.games.snake.board.up"
      :class="[BUTTON_CLASS, 'col-start-2']"
      @pointerdown="press(SnakeDirection.UP, $event)"
    >
      {{ ARROWS.up }}
    </button>
    <button
      type="button"
      :aria-label="t.games.snake.board.left"
      :class="[BUTTON_CLASS, 'col-start-1 row-start-2']"
      @pointerdown="press(SnakeDirection.LEFT, $event)"
    >
      {{ ARROWS.left }}
    </button>
    <button
      type="button"
      :aria-label="t.games.snake.board.down"
      :class="[BUTTON_CLASS, 'col-start-2 row-start-2']"
      @pointerdown="press(SnakeDirection.DOWN, $event)"
    >
      {{ ARROWS.down }}
    </button>
    <button
      type="button"
      :aria-label="t.games.snake.board.right"
      :class="[BUTTON_CLASS, 'col-start-3 row-start-2']"
      @pointerdown="press(SnakeDirection.RIGHT, $event)"
    >
      {{ ARROWS.right }}
    </button>
  </div>
</template>
