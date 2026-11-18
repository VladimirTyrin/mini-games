<script setup lang="ts">
import GameIcon from "@/components/GameIcon.vue";
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import HowToPlay from "@/components/HowToPlay.vue";
import HowToPlayButton from "@/components/HowToPlayButton.vue";
import OpponentsCard from "@/components/OpponentsCard.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import { pluginById } from "@/games";
import { controlsHintFor } from "@/games/game-plugin";
import { initialSettings } from "@/games/initial-settings";
import type { BotKind } from "@/proto/minigames/v2/game_pb";
import { type LocalSeat, useMatchStore } from "@/stores/match";
import { useConnectionStore } from "@/stores/connection";
import { usePrefsStore } from "@/stores/prefs";
import { type SettingsProblem, botKinds, gameCatalog } from "@/wasm/core";

const props = defineProps<{ gameId: string }>();
const router = useRouter();
const prefs = usePrefsStore();
const connection = useConnectionStore();
const match = useMatchStore();

const plugin = pluginById(props.gameId);
const info = gameCatalog().find((game) => game.id === props.gameId);
const settings = ref(plugin ? initialSettings(plugin, prefs.lastSettings(props.gameId)) : null);
const availableBots: BotKind[] = info?.hasBots ? botKinds(props.gameId) : [];
const bots = ref<BotKind[]>(availableBots[0] && info && info.minPlayers > 1 ? [availableBots[0]] : []);
const problem = ref<SettingsProblem | null>(null);
const helpOpen = ref(false);

// Remembered as they change, not only on Play: leaving the screen (or an app update reloading
// it) keeps what the player picked.
watch(settings, (picked) => {
  if (plugin && picked) prefs.rememberSettings(props.gameId, picked);
});

/** Watching: no seat for the person here, the bots play alone. */
const watching = ref(false);
const seats = computed<LocalSeat[]>(() => [
  ...(watching.value ? [] : [{ bot: null }]),
  ...bots.value.map((bot) => ({ bot })),
]);
const seatCountOk = computed(() => !!info && seats.value.length >= info.minPlayers);

function start(): void {
  if (!plugin || !settings.value) return;
  prefs.rememberSettings(props.gameId, settings.value);
  match.startLocal(settings.value, seats.value, connection.playerName);
}
</script>

<template>
  <AppHeader :title="plugin?.title ?? 'Unknown game'" back="/">
    <HowToPlayButton v-if="plugin" @open="helpOpen = true" />
  </AppHeader>
  <HowToPlay v-if="helpOpen && plugin" :plugin="plugin" @close="helpOpen = false" />
  <main
    v-if="plugin && info && settings"
    class="mx-auto w-full max-w-lg flex-1 space-y-4 p-4 max-[359px]:p-3"
  >
    <section class="rounded-2xl bg-panel px-4 pt-4 max-[359px]:px-3">
      <header class="flex items-start gap-3 border-b border-white/5 pb-3">
        <GameIcon :plugin="plugin" class="size-12 text-3xl" />
        <div class="min-w-0">
          <h2 class="font-semibold leading-tight">{{ plugin.tagline }}</h2>
          <p class="mt-1 text-xs leading-snug text-gray-400">{{ controlsHintFor(plugin) }}</p>
          <button
            type="button"
            class="mt-2 inline-flex min-h-9 items-center gap-2 rounded-full bg-emerald-600/15 px-3 text-sm font-medium whitespace-nowrap text-emerald-300 ring-1 ring-emerald-500/40 hover:bg-emerald-600/25"
            @click="helpOpen = true"
          >
            <span
              class="flex size-5 items-center justify-center rounded-full bg-emerald-500/25 text-xs font-bold"
              aria-hidden="true"
              >?</span
            >
            How to play
          </button>
        </div>
      </header>
      <SettingsEditor
        v-model="settings"
        :plugin="plugin"
        :seats="seats.length"
        :audience="watching ? 'watching' : 'local'"
        @validity="problem = $event"
      />
    </section>
    <OpponentsCard
      v-if="availableBots.length"
      v-model="bots"
      v-model:watching="watching"
      :plugin="plugin"
      :available="availableBots"
      :min-players="info.minPlayers"
      :max-players="info.maxPlayers"
    />
    <!-- Pinned to the bottom of the screen: on a phone the settings are taller than the screen,
         and Play must not need a scroll. The fade shows that more scrolls beneath it. -->
    <div
      class="sticky z-10 -mx-4 bg-linear-to-t from-surface from-70% to-transparent px-4 pt-4 pb-3 max-[359px]:-mx-3 max-[359px]:px-3"
      style="bottom: env(safe-area-inset-bottom)"
    >
      <button
        class="h-13 w-full rounded-xl bg-emerald-600 text-lg font-semibold shadow-lg shadow-black/40 hover:bg-emerald-500 disabled:bg-emerald-900 disabled:text-white/60"
        :disabled="!!problem || !seatCountOk"
        @click="start"
      >
        {{ seatCountOk ? "Play" : `Needs ${info.minPlayers} players` }}
      </button>
    </div>
    <button class="h-11 w-full text-sm text-gray-400 hover:text-gray-200" @click="router.push('/online')">
      Play with people instead →
    </button>
  </main>
</template>
