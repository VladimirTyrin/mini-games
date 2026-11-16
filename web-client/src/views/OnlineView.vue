<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import AppHeader from "@/components/AppHeader.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import { gamePlugins, pluginBySettings } from "@/games";
import { ChatScope, LobbyPhase } from "@/proto/minigames/v2/lobby_pb";
import { useConnectionStore } from "@/stores/connection";
import { useLobbyStore } from "@/stores/lobby";
import { usePrefsStore } from "@/stores/prefs";
import { defaultSettings, gameCatalog } from "@/wasm/core";

const connection = useConnectionStore();
const lobby = useLobbyStore();
const prefs = usePrefsStore();
const catalog = gameCatalog();

const creating = ref(false);
const gameId = ref(gamePlugins[0]?.id ?? "snake");
const settings = ref(defaultSettings(gameId.value));
const lobbyName = ref("");
const problem = ref<string | null>(null);
const plugin = computed(() => gamePlugins.find((candidate) => candidate.id === gameId.value));
const maxSeats = computed(() => catalog.find((game) => game.id === gameId.value)?.maxPlayers ?? 2);

watch(gameId, (id) => (settings.value = prefs.lastSettings(id) ?? defaultSettings(id)));
watch(
  () => connection.isSignedIn,
  (signedIn) => signedIn && void lobby.refresh(),
);
onMounted(() => connection.isSignedIn && void lobby.refresh());

async function create(): Promise<void> {
  prefs.rememberSettings(gameId.value, settings.value);
  await lobby.create(lobbyName.value, settings.value, maxSeats.value).catch(() => undefined);
  creating.value = false;
}

const phaseLabel: Record<number, string> = {
  [LobbyPhase.OPEN]: "open",
  [LobbyPhase.IN_MATCH]: "playing",
  [LobbyPhase.POST_MATCH]: "finished",
  [LobbyPhase.REPLAY]: "watching replay",
};
</script>

<template>
  <AppHeader title="Online" back="/">
    <button v-if="connection.isSignedIn" class="text-sm text-gray-400" @click="connection.signOut()">
      Sign out
    </button>
  </AppHeader>
  <main class="mx-auto w-full max-w-3xl flex-1 space-y-4 p-4">
    <SignInPanel v-if="!connection.isSignedIn" />
    <template v-else>
      <div class="flex items-center justify-between">
        <h2 class="text-sm uppercase tracking-wide text-gray-400">Lobbies</h2>
        <div class="flex gap-2">
          <button class="rounded bg-panel px-3 py-1 text-sm" @click="lobby.refresh()">Refresh</button>
          <button
            class="rounded bg-emerald-600 px-3 py-1 text-sm font-semibold"
            @click="creating = !creating"
          >
            + New lobby
          </button>
        </div>
      </div>
      <section v-if="creating && plugin" class="space-y-3 rounded-xl bg-panel p-4">
        <div class="flex flex-wrap gap-2">
          <button
            v-for="candidate in gamePlugins"
            :key="candidate.id"
            class="rounded-full px-3 py-1 text-sm"
            :class="candidate.id === gameId ? 'bg-emerald-600' : 'bg-panel-raised'"
            @click="gameId = candidate.id"
          >
            {{ candidate.icon }} {{ candidate.title }}
          </button>
        </div>
        <input
          v-model="lobbyName"
          maxlength="48"
          placeholder="Lobby name (optional)"
          class="w-full rounded bg-panel-raised px-3 py-2 text-sm"
        />
        <SettingsEditor v-model="settings" :plugin="plugin" :seats="1" @validity="problem = $event" />
        <button
          class="w-full rounded bg-emerald-600 py-2 font-semibold"
          :disabled="!!problem"
          @click="create"
        >
          Create
        </button>
      </section>
      <ul class="space-y-2">
        <li
          v-for="summary in lobby.lobbies"
          :key="summary.lobbyId"
          class="flex items-center gap-3 rounded-xl bg-panel p-3"
        >
          <span class="text-2xl">{{ pluginBySettings(summary.settings?.game.case)?.icon }}</span>
          <span class="min-w-0 flex-1">
            <span class="block truncate font-semibold">{{ summary.name }}</span>
            <span class="block text-xs text-gray-400">
              {{ summary.seatsTaken }}/{{ summary.maxSeats }} players · {{ summary.observers }} watching ·
              {{ phaseLabel[summary.phase] }}
            </span>
          </span>
          <button
            v-if="summary.phase !== LobbyPhase.IN_MATCH && summary.seatsTaken < summary.maxSeats"
            class="rounded bg-emerald-600 px-3 py-1 text-sm"
            @click="lobby.join(summary.lobbyId, false)"
          >
            Join
          </button>
          <button
            class="rounded bg-panel-raised px-3 py-1 text-sm"
            @click="lobby.join(summary.lobbyId, true)"
          >
            Watch
          </button>
        </li>
        <li v-if="!lobby.lobbies.length" class="rounded-xl bg-panel p-6 text-center text-gray-400">
          No lobbies yet — create one!
        </li>
      </ul>
      <ChatPanel :messages="lobby.lobbyListChat" @send="lobby.sendChat(ChatScope.LOBBY_LIST, $event)" />
    </template>
  </main>
</template>
