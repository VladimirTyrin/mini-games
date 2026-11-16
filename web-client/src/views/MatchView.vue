<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import MatchOverlay from "@/components/MatchOverlay.vue";
import ReplayBar from "@/components/ReplayBar.vue";
import { pluginBySettings } from "@/games";
import { controlsHintFor } from "@/games/game-plugin";
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
const endReason = computed(() => {
  const view = controller.value?.current.value?.view;
  return view ? (plugin.value?.describeEnd?.(view) ?? null) : null;
});

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

/** A running match is easy to quit by accident (back gesture, header arrow): ask first. */
const confirmingLeave = ref(false);
const leaveMessage = computed(() =>
  isOnline.value
    ? "The others will play on without you, and your seat may be dropped."
    : "This game will be lost.",
);

function requestLeave(): void {
  const current = controller.value;
  const running = current && current.source.kind !== "replay" && !current.ended.value;
  if (running) confirmingLeave.value = true;
  else void leave();
}

async function leave(): Promise<void> {
  confirmingLeave.value = false;
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
  <AppHeader :title="plugin?.title ?? 'Match'" :back="requestLeave" />
  <ConfirmDialog
    v-if="confirmingLeave"
    title="Leave the match?"
    :message="leaveMessage"
    confirm-label="Leave"
    cancel-label="Stay"
    @confirm="leave"
    @cancel="confirmingLeave = false"
  />
  <main v-if="controller && plugin" class="relative mx-auto flex w-full max-w-5xl flex-1 flex-col gap-3 p-3">
    <div class="relative flex min-h-0 flex-1 flex-col">
      <component :is="plugin.Board" :controller="controller" class="min-h-0 flex-1" />
      <MatchOverlay
        :controller="controller"
        :end-reason="endReason"
        @leave="leave"
        @replay="match.saveReplay(plugin.title)"
        @again="again"
      />
    </div>
    <ReplayBar v-if="match.replay" :replay="match.replay" />
    <p class="text-center text-xs text-gray-500">{{ controlsHintFor(plugin) }}</p>
    <ChatPanel
      v-if="isOnline && lobby.current"
      :messages="lobby.lobbyChat"
      :players="lobby.members"
      @send="lobby.sendChat(ChatScope.LOBBY, $event)"
    />
  </main>
</template>
