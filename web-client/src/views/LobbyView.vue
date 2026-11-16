<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import Avatar from "@/components/Avatar.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import { pluginBySettings } from "@/games";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ChatScope, LobbyPhase } from "@/proto/minigames/v2/lobby_pb";
import { useConnectionStore } from "@/stores/connection";
import { sameIdentity, useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";
import { usePrefsStore } from "@/stores/prefs";
import { botKinds, gameCatalog } from "@/wasm/core";

const router = useRouter();
const connection = useConnectionStore();
const lobby = useLobbyStore();
const match = useMatchStore();
const prefs = usePrefsStore();

const state = computed(() => lobby.current);
const plugin = computed(() => pluginBySettings(state.value?.settings?.game.case));
const facts = computed(() =>
  plugin.value && state.value?.settings ? plugin.value.describeSettings(state.value.settings) : [],
);
const info = computed(() => gameCatalog().find((game) => game.id === plugin.value?.id));
const availableBots = computed(() => (plugin.value && info.value?.hasBots ? botKinds(plugin.value.id) : []));
const mySeat = computed(() => (lobby.mySeatIndex >= 0 ? state.value?.seats[lobby.mySeatIndex] : undefined));
const humanSeats = computed(() => state.value?.seats.filter((seat) => !seat.botKind).length ?? 0);
const canStart = computed(
  () =>
    lobby.isHost &&
    !!state.value &&
    (state.value.phase === LobbyPhase.OPEN || state.value.phase === LobbyPhase.POST_MATCH) &&
    state.value.seats.length >= (info.value?.minPlayers ?? 1),
);

/** Nobody else is here: play locally instead (no ping, works offline). */
const startsLocally = computed(() => humanSeats.value === 1 && (state.value?.observers.length ?? 0) === 0);

async function start(): Promise<void> {
  const current = state.value;
  if (!current?.settings) return;
  if (startsLocally.value) {
    const seats = current.seats.map((seat) => ({ bot: seat.botKind ?? null }));
    const settings = current.settings;
    await lobby.leave();
    match.startLocal(settings, seats, prefs.guestName || connection.identity?.displayName || "You");
    return;
  }
  await lobby.start().catch(() => undefined);
}

async function leave(): Promise<void> {
  await lobby.leave();
  void router.push("/online");
}
</script>

<template>
  <AppHeader :title="state?.name ?? 'Lobby'" :back="leave" />
  <main
    v-if="state && plugin"
    class="mx-auto grid w-full max-w-4xl flex-1 gap-4 p-4 md:grid-cols-[1fr_20rem]"
  >
    <div class="space-y-4">
      <section class="rounded-xl bg-panel p-4">
        <div class="mb-2 flex items-center gap-2">
          <span class="text-2xl">{{ plugin.icon }}</span>
          <span class="font-semibold">{{ plugin.title }}</span>
        </div>
        <ul class="flex flex-wrap gap-2 text-xs text-gray-300">
          <li v-for="fact in facts" :key="fact" class="rounded bg-panel-raised px-2 py-1">{{ fact }}</li>
        </ul>
      </section>

      <section class="space-y-2 rounded-xl bg-panel p-4">
        <h2 class="text-sm text-gray-400">Players {{ state.seats.length }}/{{ state.maxSeats }}</h2>
        <div
          v-for="(seat, index) in state.seats"
          :key="index"
          class="flex items-center gap-2 rounded bg-panel-raised px-3 py-2"
        >
          <Avatar v-if="seat.occupant" :url="seat.occupant.avatarUrl" :name="seat.occupant.displayName" />
          <span class="flex-1 truncate">
            {{ seat.occupant?.displayName }}
            <span v-if="seat.botKind" class="text-xs text-gray-400"
              >({{ plugin.botLabel(seat.botKind) }})</span
            >
            <span v-if="sameIdentity(seat.occupant, state.host)" class="text-xs text-amber-300"> host</span>
            <span v-if="seat.connectionLost" class="text-xs text-rose-300"> reconnecting…</span>
          </span>
          <span
            v-if="!seat.botKind"
            class="text-xs"
            :class="seat.ready ? 'text-emerald-300' : 'text-gray-500'"
          >
            {{ seat.ready ? "ready" : "not ready" }}
          </span>
          <button
            v-if="lobby.isHost && seat.botKind"
            class="text-xs text-gray-400 hover:text-rose-300"
            @click="lobby.removeSeat(index)"
          >
            remove
          </button>
          <button
            v-else-if="
              lobby.isHost &&
              seat.occupant &&
              !sameIdentity(seat.occupant, connection.identity) &&
              seat.occupant.kind !== IdentityKind.BUILT_IN_BOT
            "
            class="text-xs text-gray-400 hover:text-rose-300"
            @click="lobby.kick(seat.occupant)"
          >
            kick
          </button>
        </div>
        <div
          v-if="lobby.isHost && availableBots.length && state.seats.length < state.maxSeats"
          class="flex flex-wrap gap-2 pt-1"
        >
          <button
            v-for="kind in availableBots"
            :key="plugin.botLabel(kind)"
            class="rounded bg-panel-raised px-3 py-1 text-sm"
            @click="lobby.addBot(kind)"
          >
            + {{ plugin.botLabel(kind) }}
          </button>
        </div>
        <p v-if="state.observers.length" class="pt-2 text-xs text-gray-400">
          Watching: {{ state.observers.map((observer) => observer.displayName).join(", ") }}
        </p>
      </section>

      <section class="flex flex-wrap gap-2">
        <button
          v-if="mySeat && !lobby.isHost"
          class="rounded bg-panel px-4 py-2"
          @click="lobby.setReady(!mySeat.ready)"
        >
          {{ mySeat.ready ? "Not ready" : "Ready" }}
        </button>
        <button v-if="mySeat" class="rounded bg-panel px-4 py-2 text-sm" @click="lobby.switchToObserver()">
          Watch instead
        </button>
        <button v-else class="rounded bg-panel px-4 py-2 text-sm" @click="lobby.switchToPlayer()">
          Take a seat
        </button>
        <button
          v-if="state.phase === LobbyPhase.POST_MATCH && mySeat"
          class="rounded bg-emerald-700 px-4 py-2"
          @click="lobby.votePlayAgain()"
        >
          Play again
        </button>
        <button v-if="canStart" class="ml-auto rounded bg-emerald-600 px-6 py-2 font-semibold" @click="start">
          {{ startsLocally ? "Start (offline)" : "Start" }}
        </button>
      </section>
    </div>
    <ChatPanel :messages="lobby.lobbyChat" @send="lobby.sendChat(ChatScope.LOBBY, $event)" />
  </main>
</template>
