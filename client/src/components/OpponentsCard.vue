<script setup lang="ts">
import { computed } from "vue";

import Avatar from "@/components/Avatar.vue";
import type { GamePlugin } from "@/games/game-plugin";
import type { BotKind } from "@/proto/minigames/v2/game_pb";
import { useConnectionStore } from "@/stores/connection";

/**
 * The bots of a local match. The person on this device holds the first seat, unless watching:
 * then the bots play alone.
 */
const bots = defineModel<BotKind[]>({ required: true });
const watching = defineModel<boolean>("watching", { default: false });
const props = defineProps<{
  plugin: GamePlugin;
  available: readonly BotKind[];
  minPlayers: number;
  maxPlayers: number;
}>();

const connection = useConnectionStore();
/** The signed-in account when there is one (with its avatar), otherwise the guest name. */
const me = computed(() => {
  const identity = connection.identity;
  return {
    name: connection.playerName,
    avatarUrl: identity?.avatarUrl ?? "",
    playerId: identity?.id ?? "",
  };
});

const players = computed(() => (watching.value ? 0 : 1) + bots.value.length);
const canAdd = computed(() => props.available.length > 0 && players.value < props.maxPlayers);
const canRemove = computed(() => players.value > props.minPlayers);
const hasChoice = computed(() => props.available.length > 1);

function add(): void {
  const first = props.available[0];
  if (first) bots.value = [...bots.value, first];
}

function replace(index: number, event: Event): void {
  const kind = props.available[(event.target as HTMLSelectElement).selectedIndex];
  if (kind) bots.value = bots.value.map((bot, position) => (position === index ? kind : bot));
}

/** Watching needs bots enough to play: at least the game's minimum, and one. */
function toggleWatching(): void {
  // The model reads the parent's value until it comes back: decide on the new one.
  const watchingNow = !watching.value;
  watching.value = watchingNow;
  if (!watchingNow) return;
  const first = props.available[0];
  const missing = Math.min(props.maxPlayers, Math.max(1, props.minPlayers)) - bots.value.length;
  if (first && missing > 0) bots.value = [...bots.value, ...Array<BotKind>(missing).fill(first)];
}

function remove(index: number): void {
  bots.value = bots.value.filter((_, position) => position !== index);
}
</script>

<template>
  <section class="space-y-2 rounded-2xl bg-panel p-4">
    <header class="flex min-h-7 items-center justify-between gap-2">
      <h2 class="font-semibold">Players</h2>
      <div class="flex items-center gap-2 text-xs">
        <span v-if="!bots.length" class="rounded-full bg-sky-500/20 px-2 py-0.5 font-medium text-sky-200">
          Solo
        </span>
        <span class="tabular-nums text-gray-400">{{ players }} / {{ maxPlayers }} players</span>
      </div>
    </header>
    <!-- As in a lobby: a spectator holds no seat in the list, and is named under it. -->
    <div v-if="!watching" class="flex h-12 items-center gap-3 rounded-lg bg-panel-raised/50 px-3">
      <Avatar :url="me.avatarUrl" :name="me.name" :player-id="me.playerId || undefined" />
      <span class="min-w-0 flex-1 truncate">
        {{ me.name }} <span class="text-sm text-gray-400">(you)</span>
      </span>
    </div>
    <div v-for="(bot, index) in bots" :key="index" class="flex items-center gap-2">
      <label
        class="relative flex h-12 min-w-0 flex-1 items-center gap-3 rounded-lg bg-panel-raised px-3"
        :class="hasChoice ? 'cursor-pointer hover:bg-white/10' : ''"
      >
        <Avatar url="" :name="plugin.botLabel(bot)" bot />
        <span class="flex-1 truncate">{{ plugin.botLabel(bot) }}</span>
        <span v-if="hasChoice" class="text-gray-400" aria-hidden="true">▾</span>
        <!-- The native picker sits invisibly over the row: phones get their own wheel or sheet.
             Its list gets explicit colours: some browsers (the desktop app's) open it white
             despite the dark colour scheme. -->
        <select
          v-if="hasChoice"
          class="absolute inset-0 cursor-pointer bg-[#1f2937] text-[#f3f4f6] opacity-0"
          :aria-label="`Opponent ${index + 1}`"
          @change="replace(index, $event)"
        >
          <option
            v-for="kind in available"
            :key="plugin.botLabel(kind)"
            class="bg-[#1f2937] text-[#f3f4f6]"
            :selected="plugin.botLabel(kind) === plugin.botLabel(bot)"
          >
            {{ plugin.botLabel(kind) }}
          </option>
        </select>
      </label>
      <button
        type="button"
        class="flex size-11 shrink-0 items-center justify-center rounded-lg text-lg text-gray-400 hover:bg-rose-500/15 hover:text-rose-300"
        :aria-label="`Remove ${plugin.botLabel(bot)}`"
        :disabled="!canRemove"
        @click="remove(index)"
      >
        ✕
      </button>
    </div>
    <button
      v-if="canAdd"
      type="button"
      class="flex h-12 w-full items-center justify-center gap-2 rounded-lg border border-dashed border-white/20 text-emerald-300 hover:border-emerald-400/60 hover:bg-emerald-500/5"
      @click="add"
    >
      + Add opponent
    </button>
    <p v-if="watching" class="pt-1 text-xs text-gray-400">
      Watching: {{ me.name }} <span class="text-sky-300">(you)</span>
    </p>
    <button
      v-if="available.length"
      type="button"
      class="min-h-10 rounded bg-panel-raised px-4 py-2 text-sm whitespace-nowrap"
      @click="toggleWatching"
    >
      {{ watching ? "Take a seat" : "Watch instead" }}
    </button>
  </section>
</template>
