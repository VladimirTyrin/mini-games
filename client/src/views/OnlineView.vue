<script setup lang="ts">
import GameIcon from "@/components/GameIcon.vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import HowToPlay from "@/components/HowToPlay.vue";
import SettingsEditor from "@/components/SettingsEditor.vue";
import SignInPanel from "@/components/SignInPanel.vue";
import { gamePlugins, pluginBySettings } from "@/games";
import { initialSettings } from "@/games/initial-settings";
import { ChatScope, LobbyPhase } from "@/proto/minigames/v2/lobby_pb";
import { useConnectionStore } from "@/stores/connection";
import { useFriendsStore } from "@/stores/friends";
import { useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";
import { usePrefsStore } from "@/stores/prefs";
import { type SettingsProblem, defaultSettings, gameCatalog } from "@/wasm/core";

const connection = useConnectionStore();
const friends = useFriendsStore();
const router = useRouter();
const match = useMatchStore();
const lobby = useLobbyStore();
const prefs = usePrefsStore();
const catalog = gameCatalog();

const creating = ref(false);
const gameId = ref(gamePlugins[0]?.id ?? "snake");
const settings = ref(defaultSettings(gameId.value));
const lobbyName = ref("");
const problem = ref<SettingsProblem | null>(null);
/** The full settings show only on request, so the form stays short and the list in sight. */
const editingSettings = ref(false);
const helpOpen = ref(false);
const settingsSummary = computed(() => {
  const facts = plugin.value?.describeSettings(settings.value, minSeats.value) ?? [];
  return facts.length > 0 ? facts.join(" · ") : "Standard settings";
});
// Settings that no longer pass (a remembered value the game now refuses) need fixing first.
watch(problem, (current) => {
  if (current) editingSettings.value = true;
});
const plugin = computed(() => gamePlugins.find((candidate) => candidate.id === gameId.value));
const gameInfo = computed(() => catalog.find((game) => game.id === gameId.value));
const maxSeats = computed(() => gameInfo.value?.maxPlayers ?? 2);
// The lobby fills up later; settings must already be valid for the smallest table.
const minSeats = computed(() => gameInfo.value?.minPlayers ?? 1);

watch(
  plugin,
  (current) => current && (settings.value = initialSettings(current, prefs.lastSettings(current.id))),
  { immediate: true },
);
watch(
  () => connection.isSignedIn,
  (signedIn) => signedIn && void lobby.refresh(),
);
onMounted(() => connection.isSignedIn && void lobby.refresh());

function openCurrent(): void {
  void router.push(match.controller ? { name: "match" } : { name: "lobby" });
}

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
  <HowToPlay v-if="helpOpen && plugin" :plugin="plugin" @close="helpOpen = false" />
  <main class="mx-auto w-full max-w-3xl flex-1 space-y-4 p-4 lg:max-w-6xl">
    <SignInPanel v-if="!connection.isSignedIn" />
    <div
      v-else
      class="flex flex-col gap-4 lg:grid lg:items-start lg:gap-6"
      :class="lobby.current ? '' : 'lg:grid-cols-[minmax(0,1fr)_28rem]'"
    >
      <div class="contents lg:flex lg:min-w-0 lg:flex-col lg:gap-4">
        <!-- Reachable while in a lobby (from Home, or back from Friends): one lobby at a time, so
           the way on is back into it. -->
        <button
          v-if="lobby.current"
          class="flex min-h-12 w-full items-center gap-2 rounded-xl bg-emerald-700 px-4 text-left font-semibold"
          @click="openCurrent"
        >
          <span class="min-w-0 flex-1 truncate">You are in {{ lobby.current.name || "a lobby" }}</span>
          <span class="shrink-0">Open ›</span>
        </button>
        <RouterLink
          v-if="friends.hasAccount"
          to="/friends"
          class="flex min-h-12 items-center gap-2 rounded-xl bg-panel px-4 font-semibold"
        >
          Friends
          <span
            v-if="friends.incomingRequestCount"
            class="rounded-full bg-rose-600 px-2 text-xs font-semibold"
          >
            {{ friends.incomingRequestCount }} new
          </span>
          <span class="ml-auto text-gray-400" aria-hidden="true">›</span>
        </RouterLink>
        <div class="flex items-center justify-between">
          <h2 class="text-sm uppercase tracking-wide text-gray-400">Lobbies</h2>
          <div class="flex gap-2">
            <button class="min-h-10 rounded bg-panel px-3 py-2 text-sm" @click="lobby.refresh()">
              Refresh
            </button>
            <button
              v-if="!lobby.current"
              class="min-h-10 rounded bg-emerald-600 px-3 py-2 text-sm font-semibold whitespace-nowrap lg:hidden"
              @click="creating = !creating"
            >
              + New lobby
            </button>
          </div>
        </div>
        <ul class="order-2 space-y-2 lg:order-none">
          <li
            v-for="summary in lobby.lobbies"
            :key="summary.lobbyId"
            class="flex items-center gap-3 rounded-xl bg-panel p-3"
          >
            <GameIcon
              v-if="pluginBySettings(summary.settings?.game.case)"
              :plugin="pluginBySettings(summary.settings?.game.case)!"
              class="size-9 text-2xl"
            />
            <span class="min-w-0 flex-1">
              <span class="block truncate font-semibold">{{ summary.name }}</span>
              <span v-if="summary.host" class="flex items-center gap-1 text-xs text-gray-300">
                <Avatar
                  :url="summary.host.avatarUrl"
                  :name="summary.host.displayName"
                  :player-id="summary.host.id"
                />
                <span class="truncate">{{ summary.host.displayName }}</span>
              </span>
              <span class="block text-xs text-gray-400">
                {{ summary.seatsTaken }}/{{ summary.maxSeats }} players · {{ summary.observers }} watching ·
                {{ phaseLabel[summary.phase] }}
              </span>
            </span>
            <button
              v-if="summary.lobbyId === lobby.current?.lobbyId"
              class="min-h-10 rounded bg-emerald-600 px-3 py-2 text-sm"
              @click="openCurrent"
            >
              Open
            </button>
            <template v-else-if="!lobby.current">
              <button
                v-if="summary.phase !== LobbyPhase.IN_MATCH && summary.seatsTaken < summary.maxSeats"
                class="min-h-10 rounded bg-emerald-600 px-3 py-2 text-sm"
                @click="lobby.join(summary.lobbyId, false)"
              >
                Join
              </button>
              <button
                class="min-h-10 rounded bg-panel-raised px-3 py-2 text-sm"
                @click="lobby.join(summary.lobbyId, true)"
              >
                Watch
              </button>
            </template>
          </li>
          <li v-if="!lobby.lobbies.length" class="rounded-xl bg-panel p-6 text-center text-gray-400">
            No lobbies yet — create one!
          </li>
        </ul>
        <ChatPanel
          class="order-2 lg:order-none"
          :messages="lobby.lobbyListChat"
          @send="lobby.sendChat(ChatScope.LOBBY_LIST, $event)"
        />
      </div>
      <!-- On a phone the form opens under "+ New lobby" and above the list; on a wide screen it
           stands beside the list, settings and all. -->
      <section
        v-if="plugin && !lobby.current"
        class="order-1 space-y-3 rounded-2xl bg-panel p-4 lg:order-none"
        :class="creating ? '' : 'hidden lg:block'"
      >
        <h2 class="font-semibold">New lobby</h2>
        <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Game">
          <button
            v-for="candidate in gamePlugins"
            :key="candidate.id"
            role="radio"
            :aria-checked="candidate.id === gameId"
            class="h-11 rounded-full border px-4 text-sm"
            :class="
              candidate.id === gameId
                ? 'border-emerald-400 bg-emerald-600/25 text-white'
                : 'border-white/10 bg-panel-raised text-gray-200'
            "
            @click="gameId = candidate.id"
          >
            <GameIcon :plugin="candidate" class="size-6 text-base" /> {{ candidate.title }}
          </button>
        </div>
        <div class="flex items-center gap-3 border-b border-white/5 pb-3">
          <p class="min-w-0 flex-1 font-medium leading-tight">{{ plugin.tagline }}</p>
          <button
            type="button"
            class="shrink-0 text-sm whitespace-nowrap text-emerald-300 hover:text-emerald-200"
            @click="helpOpen = true"
          >
            How to play
          </button>
        </div>
        <input
          v-model="lobbyName"
          maxlength="48"
          placeholder="Lobby name (optional)"
          aria-label="Lobby name"
          class="h-12 w-full rounded-lg bg-panel-raised px-3 text-[15px]"
        />
        <div class="flex items-center gap-3 rounded-xl bg-panel-raised px-3 py-2 lg:hidden">
          <p class="min-w-0 flex-1 text-sm text-gray-300">{{ settingsSummary }}</p>
          <button
            type="button"
            class="min-h-10 shrink-0 rounded-lg px-2 text-sm whitespace-nowrap text-emerald-300 hover:bg-white/5"
            :aria-expanded="editingSettings"
            @click="editingSettings = !editingSettings"
          >
            {{ editingSettings ? "Hide settings" : "Change settings" }}
          </button>
        </div>
        <!-- Kept mounted while hidden: it reports whether the settings pass. -->
        <div :class="editingSettings ? '' : 'hidden lg:block'">
          <SettingsEditor
            v-model="settings"
            :plugin="plugin"
            :seats="minSeats"
            audience="online"
            @validity="problem = $event"
          />
        </div>
        <button
          class="h-12 w-full rounded-xl bg-emerald-600 font-semibold hover:bg-emerald-500 disabled:opacity-50"
          :disabled="!!problem"
          @click="create"
        >
          Create lobby
        </button>
      </section>
    </div>
  </main>
</template>
