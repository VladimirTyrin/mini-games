<script setup lang="ts">
import { computed } from "vue";

import { SnakeDeathReason, type SnakeView } from "@/proto/minigames/v2/snake_pb";

import { DEAD_SNAKE_COLOR, cssColor, snakeColor } from "./snake-palette";

const props = defineProps<{ view: SnakeView; seatNames: readonly string[]; status: string }>();

const DEATH_NOTES: Partial<Record<SnakeDeathReason, string>> = {
  [SnakeDeathReason.WALL_COLLISION]: "hit a wall",
  [SnakeDeathReason.SELF_COLLISION]: "bit itself",
  [SnakeDeathReason.SNAKE_COLLISION]: "crashed",
  [SnakeDeathReason.PLAYER_DISCONNECTED]: "left",
};

const entries = computed(() =>
  [...props.view.snakes]
    .sort((first, second) => second.score - first.score || first.slot - second.slot)
    .map((seat) => ({
      slot: seat.slot,
      name: props.seatNames[seat.slot] ?? `Player ${seat.slot + 1}`,
      isBot: seat.isBot,
      isYou: seat.slot === props.view.you,
      isWinner: props.view.end?.winnerSlots.includes(seat.slot) ?? false,
      alive: seat.alive,
      deathNote: seat.alive ? "" : (DEATH_NOTES[seat.deathReason] ?? "out"),
      score: seat.score,
      color: cssColor(seat.alive ? snakeColor(seat.slot) : DEAD_SNAKE_COLOR),
    })),
);
</script>

<template>
  <aside class="flex min-h-0 flex-col gap-2 rounded-lg bg-panel p-2 md:p-3">
    <p v-if="status" class="text-center text-sm text-gray-300">{{ status }}</p>
    <ol class="grid max-h-28 grid-cols-2 gap-1 overflow-y-auto md:max-h-none md:grid-cols-1">
      <li
        v-for="entry in entries"
        :key="entry.slot"
        class="flex items-center gap-2 rounded px-2 py-1 text-sm"
        :class="entry.isYou ? 'bg-panel-raised ring-1 ring-white/40' : ''"
      >
        <span class="size-3 shrink-0 rounded-sm" :style="{ backgroundColor: entry.color }" />
        <span class="min-w-0 flex-1 truncate" :class="entry.alive ? 'text-gray-100' : 'text-gray-500'">
          <span v-if="entry.isWinner" aria-label="winner">👑 </span>
          <span :class="entry.alive ? '' : 'line-through'">{{ entry.name }}</span>
          <span v-if="entry.isYou" class="text-xs text-gray-400"> (you)</span>
          <span v-if="entry.isBot" class="text-xs text-gray-400"> bot</span>
          <span v-if="entry.deathNote" class="text-xs text-rose-300/80"> · {{ entry.deathNote }}</span>
        </span>
        <span class="font-mono font-semibold" :class="entry.alive ? 'text-emerald-300' : 'text-gray-500'">
          {{ entry.score }}
        </span>
      </li>
    </ol>
  </aside>
</template>
