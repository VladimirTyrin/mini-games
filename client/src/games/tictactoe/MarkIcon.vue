<script setup lang="ts">
import { TicTacToeMark } from "@/proto/minigames/v2/tictactoe_pb";

// SVG instead of the ✕/○ glyphs: fonts draw ○ much smaller than ✕.
defineProps<{ mark: TicTacToeMark }>();
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="3"
    stroke-linecap="round"
    :aria-label="mark === TicTacToeMark.X ? 'X' : 'O'"
    role="img"
  >
    <path v-if="mark === TicTacToeMark.X" d="M5 5 19 19M19 5 5 19" />
    <circle v-else cx="12" cy="12" r="7.5" />
  </svg>
</template>
