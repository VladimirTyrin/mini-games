<script setup lang="ts">
import { computed } from "vue";

import { t } from "@/i18n";
import { SnakeDeathReason, type SnakeView } from "@/proto/minigames/v2/snake_pb";

import { crashesOf } from "./snake-crash";
import { DEAD_SNAKE_COLOR, cssColor, snakeColor } from "./snake-palette";

const props = defineProps<{ view: SnakeView; seatNames: readonly string[]; status: string }>();

const WINNER_MARK = "👑";

function deathNoteOf(reason: SnakeDeathReason): string {
  const notes = t.games.snake.scoreboard;
  switch (reason) {
    case SnakeDeathReason.WALL_COLLISION:
      return notes.hitWall;
    case SnakeDeathReason.SELF_COLLISION:
      return notes.bitItself;
    case SnakeDeathReason.SNAKE_COLLISION:
      return notes.crashed;
    case SnakeDeathReason.PLAYER_DISCONNECTED:
      return notes.left;
    default:
      return notes.out;
  }
}

const headOnSlots = computed(
  () => new Set(crashesOf(props.view, null).flatMap((crash) => (crash.headOn ? crash.slots : []))),
);

const entries = computed(() =>
  [...props.view.snakes]
    .sort((first, second) => second.score - first.score || first.slot - second.slot)
    .map((seat) => ({
      slot: seat.slot,
      name: props.seatNames[seat.slot] ?? t.match.player(seat.slot + 1),
      isYou: seat.slot === props.view.you,
      isWinner: props.view.end?.winnerSlots.includes(seat.slot) ?? false,
      alive: seat.alive,
      deathNote: seat.alive
        ? ""
        : headOnSlots.value.has(seat.slot)
          ? t.games.snake.scoreboard.crashedHeadOn
          : deathNoteOf(seat.deathReason),
      score: seat.score,
      color: cssColor(seat.alive ? snakeColor(seat.slot) : DEAD_SNAKE_COLOR),
    })),
);
</script>

<template>
  <aside class="flex min-h-0 flex-col gap-2 rounded-lg bg-panel p-2 md:p-3">
    <p v-if="status" class="text-center text-sm text-gray-300">{{ status }}</p>
    <ol class="grid max-h-28 grid-cols-2 gap-1 overflow-y-auto landscape:max-h-none landscape:grid-cols-1">
      <li
        v-for="entry in entries"
        :key="entry.slot"
        class="flex items-center gap-2 rounded px-2 py-1 text-sm"
        :class="entry.isYou ? 'bg-panel-raised ring-1 ring-white/40' : ''"
      >
        <span class="size-3 shrink-0 rounded-sm" :style="{ backgroundColor: entry.color }" />
        <span class="min-w-0 flex-1 truncate" :class="entry.alive ? 'text-gray-100' : 'text-gray-500'">
          <span v-if="entry.isWinner" :aria-label="t.games.snake.scoreboard.winner">{{ WINNER_MARK }} </span>
          <span :class="entry.alive ? '' : 'line-through'">{{ entry.name }}</span>
          <span v-if="entry.isYou" class="text-xs text-gray-400"> {{ t.games.snake.scoreboard.you }}</span>
          <span v-if="entry.deathNote" class="text-xs text-rose-300/80"> · {{ entry.deathNote }}</span>
        </span>
        <span class="font-mono font-semibold" :class="entry.alive ? 'text-emerald-300' : 'text-gray-500'">
          {{ entry.score }}
        </span>
      </li>
    </ol>
  </aside>
</template>
