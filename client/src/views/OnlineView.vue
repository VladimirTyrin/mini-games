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

const gameId = ref(gamePlugins[0]?.id ?? "snake");
const settings = ref(defaultSettings(gameId.value));
const lobbyName = ref("");
const problem = ref<SettingsProblem | null>(null);
/** The full settings show only on request, so the form stays short and the list in sight. */
const editingSettings = ref(false);
const helpOpen = ref(false);
const settingsSummary = computed(() => {
  const facts = plugin.value?.describeSettings(settings.value, minSeats.value) ?? [];
  // The name sits with the folded settings: the summary keeps it in sight.
  const name = lobbyName.value.trim();
  const summary = facts.length > 0 ? facts.join(" · ") : "Standard settings";
  return name ? `“${name}” · ${summary}` : summary;
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
    <!-- Where Settings stands on the home screen; Sign out lives in Settings, out of a stray tap's
         reach. -->
    <button
      class="rounded px-2 py-1 text-sm whitespace-nowrap hover:bg-panel-raised"
      @click="router.push('/settings')"
    >
      ⚙ Settings
    </button>
  </AppHeader>
  <HowToPlay v-if="helpOpen && plugin" :plugin="plugin" @close="helpOpen = false" />
  <main class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 p-4 lg:max-w-6xl">
    <SignInPanel v-if="!connection.isSignedIn" />
    <div
      v-else
      class="flex flex-1 flex-col gap-3 lg:grid lg:items-start lg:gap-6"
      :class="lobby.current ? '' : 'lg:grid-cols-[minmax(0,1fr)_34rem] xl:grid-cols-[minmax(0,1fr)_40rem]'"
    >
      <div class="contents lg:flex lg:min-w-0 lg:flex-col lg:gap-4 lg:self-stretch">
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
        <h2 class="order-2 text-sm uppercase tracking-wide text-gray-400 lg:order-none">Lobbies</h2>
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
          <li v-if="!lobby.lobbies.length" class="rounded-xl bg-panel p-4 text-center text-gray-400">
            No lobbies yet — create one!
          </li>
        </ul>
        <!-- Its usual height when the screen has room; on a phone short of room it gives way, down
             to the input and a couple of lines, rather than make the screen scroll. -->
        <ChatPanel
          class="order-2 max-h-[17rem] min-h-24 flex-1 basis-0 lg:order-none lg:basis-auto"
          shrinks-to-fit
          :messages="lobby.lobbyListChat"
          @send="lobby.sendChat(ChatScope.LOBBY_LIST, $event)"
        />
      </div>
      <!-- Always there: on a phone above the list, on a wide screen beside it with every setting open. -->
      <section
        v-if="plugin && !lobby.current"
        class="order-1 space-y-3 rounded-2xl bg-panel p-3 sm:p-4 lg:order-none"
      >
        <!-- The games as on the home screen: two per row, low tiles on a phone, with the tagline
             where there is room. -->
        <div class="grid grid-cols-2 gap-2 sm:gap-3" role="radiogroup" aria-label="Game">
          <button
            v-for="candidate in gamePlugins"
            :key="candidate.id"
            type="button"
            role="radio"
            :aria-checked="candidate.id === gameId"
            class="flex items-center gap-2 rounded-xl p-1.5 text-left sm:gap-3 sm:p-3"
            :class="
              candidate.id === gameId
                ? 'bg-emerald-600/20 ring-2 ring-emerald-400'
                : 'bg-panel-raised hover:bg-white/10'
            "
            @click="gameId = candidate.id"
          >
            <GameIcon :plugin="candidate" class="size-7 shrink-0 text-lg sm:size-10 sm:text-2xl" />
            <span class="min-w-0">
              <span class="block text-sm font-semibold leading-tight sm:text-base">{{
                candidate.title
              }}</span>
              <span class="hidden text-xs leading-snug text-gray-400 sm:block">{{ candidate.tagline }}</span>
            </span>
          </button>
        </div>
        <div class="flex items-center gap-3 border-b border-white/5 pb-2 sm:pb-3">
          <p class="min-w-0 flex-1 font-medium leading-tight sm:hidden">{{ plugin.tagline }}</p>
          <button
            type="button"
            class="ml-auto shrink-0 text-sm whitespace-nowrap text-emerald-300 hover:text-emerald-200"
            @click="helpOpen = true"
          >
            How to play
          </button>
        </div>
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
        <div class="space-y-3" :class="editingSettings ? '' : 'hidden lg:block'">
          <input
            v-model="lobbyName"
            maxlength="48"
            placeholder="Lobby name (optional)"
            aria-label="Lobby name"
            class="h-11 w-full rounded-lg bg-panel-raised px-3 text-[15px]"
          />
          <SettingsEditor
            v-model="settings"
            :plugin="plugin"
            :seats="minSeats"
            audience="online"
            @validity="problem = $event"
          />
        </div>
        <!-- Pinned to the bottom of the screen while the settings run past it, as Play is on a
             game's setup screen: Create must not need a scroll. The fade shows more beneath. -->
        <div
          class="sticky z-10 -mx-3 -mb-3 rounded-b-2xl bg-linear-to-t from-panel from-70% to-transparent px-3 pt-3 pb-3 sm:-mx-4 sm:-mb-4 sm:px-4 sm:pb-4"
          style="bottom: env(safe-area-inset-bottom)"
        >
          <button
            class="h-12 w-full rounded-xl bg-emerald-600 font-semibold shadow-lg shadow-black/40 hover:bg-emerald-500 disabled:opacity-50"
            :disabled="!!problem"
            @click="create"
          >
            Create lobby
          </button>
        </div>
      </section>
    </div>
  </main>
</template>
