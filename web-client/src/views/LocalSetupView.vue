<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import OpponentsCard from "@/components/OpponentsCard.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import { pluginById } from "@/games";
import { initialSettings } from "@/games/initial-settings";
import type { BotKind } from "@/proto/minigames/v2/game_pb";
import { type LocalSeat, useMatchStore } from "@/stores/match";
import { usePrefsStore } from "@/stores/prefs";
import { type SettingsProblem, botKinds, gameCatalog } from "@/wasm/core";

const props = defineProps<{ gameId: string }>();
const router = useRouter();
const prefs = usePrefsStore();
const match = useMatchStore();

const plugin = pluginById(props.gameId);
const info = gameCatalog().find((game) => game.id === props.gameId);
const settings = ref(plugin ? initialSettings(plugin, prefs.lastSettings(props.gameId)) : null);
const availableBots: BotKind[] = info?.hasBots ? botKinds(props.gameId) : [];
const bots = ref<BotKind[]>(availableBots[0] && info && info.minPlayers > 1 ? [availableBots[0]] : []);
const problem = ref<SettingsProblem | null>(null);

const seats = computed<LocalSeat[]>(() => [{ bot: null }, ...bots.value.map((bot) => ({ bot }))]);
const seatCountOk = computed(() => !!info && seats.value.length >= info.minPlayers);

function start(): void {
  if (!plugin || !settings.value) return;
  prefs.rememberSettings(props.gameId, settings.value);
  match.startLocal(settings.value, seats.value, prefs.guestName);
}
</script>

<template>
  <AppHeader :title="plugin?.title ?? 'Unknown game'" back="/" />
  <main v-if="plugin && info && settings" class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4">
    <section class="rounded-2xl bg-panel px-4 pt-4">
      <header class="flex items-start gap-3 border-b border-white/5 pb-3">
        <span class="text-3xl leading-none" aria-hidden="true">{{ plugin.icon }}</span>
        <div class="min-w-0">
          <h2 class="font-semibold leading-tight">{{ plugin.tagline }}</h2>
          <p class="mt-1 text-xs leading-snug text-gray-400">{{ plugin.controlsHint }}</p>
        </div>
      </header>
      <SettingsEditor
        v-model="settings"
        :plugin="plugin"
        :seats="seats.length"
        audience="local"
        @validity="problem = $event"
      />
    </section>
    <OpponentsCard
      v-if="availableBots.length"
      v-model="bots"
      :plugin="plugin"
      :available="availableBots"
      :min-players="info.minPlayers"
      :max-players="info.maxPlayers"
    />
    <button
      class="h-13 w-full rounded-xl bg-emerald-600 text-lg font-semibold hover:bg-emerald-500 disabled:opacity-50"
      :disabled="!!problem || !seatCountOk"
      @click="start"
    >
      {{ seatCountOk ? "Play" : `Needs ${info.minPlayers} players` }}
    </button>
    <button class="h-11 w-full text-sm text-gray-400 hover:text-gray-200" @click="router.push('/online')">
      Play with people instead →
    </button>
  </main>
</template>
