<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import { pluginById } from "@/games";
import type { BotKind } from "@/proto/minigames/v2/game_pb";
import { type LocalSeat, useMatchStore } from "@/stores/match";
import { usePrefsStore } from "@/stores/prefs";
import { botKinds, defaultSettings, gameCatalog } from "@/wasm/core";

const props = defineProps<{ gameId: string }>();
const router = useRouter();
const prefs = usePrefsStore();
const match = useMatchStore();

const plugin = pluginById(props.gameId);
const info = gameCatalog().find((game) => game.id === props.gameId);
const settings = ref(prefs.lastSettings(props.gameId) ?? defaultSettings(props.gameId));
const availableBots: BotKind[] = info?.hasBots ? botKinds(props.gameId) : [];
const bots = ref<BotKind[]>(availableBots[0] && info && info.minPlayers > 1 ? [availableBots[0]] : []);
const problem = ref<string | null>(null);

const seats = computed<LocalSeat[]>(() => [{ bot: null }, ...bots.value.map((bot) => ({ bot }))]);
const canAddBot = computed(() => !!info && availableBots.length > 0 && seats.value.length < info.maxPlayers);
const seatCountOk = computed(() => !!info && seats.value.length >= info.minPlayers);

function start(): void {
  if (!plugin) return;
  prefs.rememberSettings(props.gameId, settings.value);
  match.startLocal(settings.value, seats.value, prefs.guestName);
}
</script>

<template>
  <AppHeader :title="plugin?.title ?? 'Unknown game'" back="/" />
  <main v-if="plugin && info" class="mx-auto w-full max-w-lg flex-1 space-y-5 p-4">
    <section class="space-y-3 rounded-xl bg-panel p-4">
      <h2 class="font-semibold">Settings</h2>
      <SettingsEditor
        v-model="settings"
        :plugin="plugin"
        :seats="seats.length"
        @validity="problem = $event"
      />
    </section>
    <section v-if="availableBots.length" class="space-y-3 rounded-xl bg-panel p-4">
      <h2 class="font-semibold">Opponents</h2>
      <div v-for="(bot, index) in bots" :key="index" class="flex items-center justify-between gap-2">
        <select v-model="bots[index]" class="flex-1 rounded bg-panel-raised px-2 py-1 text-sm">
          <option v-for="kind in availableBots" :key="plugin.botLabel(kind)" :value="kind">
            {{ plugin.botLabel(kind) }}
          </option>
        </select>
        <button class="rounded px-2 text-gray-400 hover:text-rose-300" @click="bots.splice(index, 1)">
          ✕
        </button>
      </div>
      <button
        v-if="canAddBot"
        class="text-sm text-emerald-300"
        @click="availableBots[0] && bots.push(availableBots[0])"
      >
        + Add bot
      </button>
    </section>
    <button
      class="w-full rounded-xl bg-emerald-600 py-3 font-semibold hover:bg-emerald-500"
      :disabled="!!problem || !seatCountOk"
      @click="start"
    >
      {{ seatCountOk ? "Play" : `Needs ${info.minPlayers} players` }}
    </button>
    <button class="w-full text-sm text-gray-400" @click="router.push('/online')">
      Play with people instead →
    </button>
  </main>
</template>
