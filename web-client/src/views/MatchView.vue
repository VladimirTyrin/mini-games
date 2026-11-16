<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import MatchOverlay from "@/components/MatchOverlay.vue";
import ReplayBar from "@/components/ReplayBar.vue";
import { pluginBySettings } from "@/games";
import { LocalMatchSource } from "@/local/local-match-source";
import { ChatScope } from "@/proto/minigames/v2/lobby_pb";
import { useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";

const router = useRouter();
const match = useMatchStore();
const lobby = useLobbyStore();

const controller = computed(() => match.controller);
const plugin = computed(() => pluginBySettings(controller.value?.source.header.settings?.game.case));
const isOnline = computed(() => controller.value?.source.kind === "online");

onMounted(() => {
  if (!controller.value) void router.replace("/");
});

// A backgrounded tab or app must not let bots play on alone.
function onVisibility(): void {
  const source = controller.value?.source;
  if (source instanceof LocalMatchSource) source.setPaused(document.hidden);
}
document.addEventListener("visibilitychange", onVisibility);
onBeforeUnmount(() => document.removeEventListener("visibilitychange", onVisibility));

async function leave(): Promise<void> {
  const kind = controller.value?.source.kind;
  match.close();
  if (kind === "online") await lobby.leave();
  void router.push(kind === "online" ? "/online" : "/");
}

function again(): void {
  const current = controller.value;
  if (!current) return;
  if (current.source.kind === "online") {
    match.close();
    void lobby.votePlayAgain().catch(() => undefined);
    void router.push("/lobby");
    return;
  }
  const header = current.source.header;
  if (!header.settings) return;
  match.startLocal(
    header.settings,
    header.seats.map((seat) => ({ bot: seat.botKind ?? null })),
    header.seats.find((seat) => !seat.botKind)?.player?.displayName ?? "You",
  );
}
</script>

<template>
  <AppHeader :title="plugin?.title ?? 'Match'" :back="leave" />
  <main v-if="controller && plugin" class="relative mx-auto flex w-full max-w-5xl flex-1 flex-col gap-3 p-3">
    <div class="relative flex min-h-0 flex-1 flex-col">
      <component :is="plugin.Board" :controller="controller" class="min-h-0 flex-1" />
      <MatchOverlay :controller="controller" @leave="leave" @replay="match.downloadReplay()" @again="again" />
    </div>
    <ReplayBar v-if="match.replay" :replay="match.replay" />
    <p class="hidden text-center text-xs text-gray-500 md:block">{{ plugin.controlsHint }}</p>
    <ChatPanel
      v-if="isOnline && lobby.current"
      :messages="lobby.lobbyChat"
      :players="lobby.members"
      @send="lobby.sendChat(ChatScope.LOBBY, $event)"
    />
  </main>
</template>
