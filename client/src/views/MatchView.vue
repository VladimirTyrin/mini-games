<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRouter } from "vue-router";

import AppHeader from "@/components/AppHeader.vue";
import ChatPanel from "@/components/ChatPanel.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import MatchOverlay from "@/components/MatchOverlay.vue";
import ReplayBar from "@/components/ReplayBar.vue";
import { pluginBySettings } from "@/games";
import { controlsHintFor } from "@/games/game-plugin";
import { LocalMatchSource } from "@/local/local-match-source";
import { setKeepScreenOn } from "@/platform/keep-awake";
import { ChatScope } from "@/proto/minigames/v2/lobby_pb";
import { useAppUpdateStore } from "@/stores/app-update";
import { useLobbyStore } from "@/stores/lobby";
import { useMatchStore } from "@/stores/match";
import { useToastStore } from "@/stores/toasts";

const router = useRouter();
const match = useMatchStore();
const lobby = useLobbyStore();
const appUpdate = useAppUpdateStore();
const toasts = useToastStore();

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

const incompatibleMessage = computed(() =>
  match.replay
    ? "This replay was recorded with another version of the game and cannot be shown."
    : "The game was updated on the server. Update the app to go on playing.",
);

// A replay this build cannot follow would play on behind the message.
watch(
  () => controller.value?.incompatible.value === true,
  (incompatible) => {
    if (incompatible) match.replay?.pause();
  },
);

/** The web build reloads into the new version; the app fetches and switches to its bundle. */
async function update(): Promise<void> {
  if (!__NATIVE_APP__) {
    window.location.reload();
    return;
  }
  // A reload would start the same old bundle again: only a newer one helps.
  const report = await appUpdate.check();
  if (report.kind === "web-bundle-ready") return;
  if (report.kind === "apk-available") await appUpdate.install();
  else toasts.show("No update is out yet. Try again in a minute.");
}

/** Set when this screen ends the match itself and navigates on its own. */
let isLeavingOnPurpose = false;
// The match can also end under this screen: the seat was lost after a long disconnect.
watch(controller, (current) => {
  if (!current && !isLeavingOnPurpose) void router.replace(lobby.current ? "/lobby" : "/online");
});

/** The online countdown's length; turn-based games just wait for the next move. */
function resumeCountdownMs(): number {
  return (controller.value?.gameTickMs ?? 0) > 0 ? 3_000 : 0;
}

// A backgrounded tab or app must not let bots play on alone.
function onVisibility(): void {
  const source = controller.value?.source;
  if (source instanceof LocalMatchSource) source.setPaused("hidden", document.hidden, resumeCountdownMs());
}
document.addEventListener("visibilitychange", onVisibility);
onBeforeUnmount(() => document.removeEventListener("visibilitychange", onVisibility));

/** A local real-time game can be paused; a turn-based one waits anyway, an online one is shared. */
const canPause = computed(
  () =>
    controller.value?.source instanceof LocalMatchSource &&
    (controller.value?.gameTickMs ?? 0) > 0 &&
    !controller.value.ended.value,
);
const pausedByPlayer = ref(false);
function setPausedByPlayer(paused: boolean): void {
  const source = controller.value?.source;
  if (!(source instanceof LocalMatchSource)) return;
  pausedByPlayer.value = paused;
  source.setPaused("player", paused, resumeCountdownMs());
}
// A new match (Play again) starts unpaused.
watch(controller, () => {
  pausedByPlayer.value = false;
});
function onPauseKey(event: KeyboardEvent): void {
  if (event.key !== "Escape" || !canPause.value || confirmingLeave.value) return;
  event.preventDefault();
  setPausedByPlayer(!pausedByPlayer.value);
}
window.addEventListener("keydown", onPauseKey);
onBeforeUnmount(() => window.removeEventListener("keydown", onPauseKey));

/** A running match is easy to quit by accident (back gesture, header arrow): ask first. */
const confirmingLeave = ref(false);
const leaveMessage = computed(() =>
  isOnline.value
    ? "The others will play on without you, and your seat may be dropped."
    : "This game will be lost.",
);

/** A match still being played, which leaving would give up. */
function isRunning(): boolean {
  const current = controller.value;
  return (
    current !== null &&
    current.source.kind !== "replay" &&
    !current.ended.value &&
    !current.incompatible.value
  );
}

function requestLeave(): void {
  if (isRunning()) confirmingLeave.value = true;
  else void leave();
}

// Leaving by the browser's back, an iOS swipe or a toast's link must not leave the match running
// unseen (bots playing on, the seat held, updates blocked): a running match asks first, a
// finished one or a replay is closed on the way out.
onBeforeRouteLeave(() => {
  if (isLeavingOnPurpose || !controller.value) return true;
  if (isRunning()) {
    confirmingLeave.value = true;
    return false;
  }
  isLeavingOnPurpose = true;
  match.close();
  return true;
});

// A reload or a closed tab loses a match being played; the browser asks first.
function onBeforeUnload(event: BeforeUnloadEvent): void {
  if (isRunning()) event.preventDefault();
}
window.addEventListener("beforeunload", onBeforeUnload);
onBeforeUnmount(() => window.removeEventListener("beforeunload", onBeforeUnload));

// Thinking over a move (or waiting for the opponent's) must not let the screen go dark.
const keepsScreenOn = computed(() => isRunning() && !pausedByPlayer.value);
watch(keepsScreenOn, (keepScreenOn) => void setKeepScreenOn(keepScreenOn), { immediate: true });
onBeforeUnmount(() => void setKeepScreenOn(false));

async function leave(): Promise<void> {
  confirmingLeave.value = false;
  const kind = controller.value?.source.kind;
  isLeavingOnPurpose = true;
  match.close();
  if (kind === "online") await lobby.leave();
  // In the app a replay is usually opened from the saved-replays list, so leave back to it.
  const home = __NATIVE_APP__ && kind === "replay" ? "/replays" : "/";
  void router.push(kind === "online" ? "/online" : home);
}

// A board times its animations by `controller.tickMs` once, when it is created; a new replay
// speed changes that, so the board is rebuilt (it draws from the controller's state).
const boardKey = ref(0);
watch(
  () => match.replay,
  (replay, _, onCleanup) => {
    if (!replay) return;
    let speed = replay.speed;
    onCleanup(
      replay.onProgress(() => {
        if (replay.speed === speed) return;
        speed = replay.speed;
        boardKey.value += 1;
      }),
    );
  },
  { immediate: true },
);

function again(): void {
  const current = controller.value;
  if (!current) return;
  if (current.source.kind === "replay") {
    match.replay?.restart();
    return;
  }
  if (current.source.kind === "online") {
    isLeavingOnPurpose = true;
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
  <AppHeader :title="plugin?.title ?? 'Match'" :back="requestLeave">
    <button
      v-if="canPause && !pausedByPlayer"
      type="button"
      class="rounded px-2 py-1 text-sm whitespace-nowrap hover:bg-panel-raised"
      @click="setPausedByPlayer(true)"
    >
      ⏸ Pause
    </button>
  </AppHeader>
  <ConfirmDialog
    v-if="confirmingLeave"
    title="Leave the match?"
    :message="leaveMessage"
    confirm-label="Leave"
    cancel-label="Stay"
    @confirm="leave"
    @cancel="confirmingLeave = false"
  />
  <main
    v-if="controller && plugin"
    class="relative mx-auto flex w-full max-w-5xl flex-1 flex-col gap-3 p-3 xl:max-w-6xl 2xl:max-w-7xl"
  >
    <div class="relative flex min-h-0 flex-1 flex-col">
      <component :is="plugin.Board" :key="boardKey" :controller="controller" class="min-h-0 flex-1" />
      <div
        v-if="controller.incompatible.value"
        class="absolute inset-0 z-20 flex flex-col items-center justify-center gap-3 rounded-xl bg-gray-950/90 p-4 text-center"
      >
        <p class="max-w-xs text-sm text-gray-200">{{ incompatibleMessage }}</p>
        <div class="flex flex-wrap justify-center gap-2">
          <button
            v-if="!match.replay"
            type="button"
            class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium whitespace-nowrap text-white hover:bg-emerald-500"
            @click="update"
          >
            Update
          </button>
          <button
            type="button"
            class="rounded-lg border border-white/15 px-4 py-2 text-sm whitespace-nowrap text-gray-200 hover:bg-white/5"
            @click="leave"
          >
            Leave
          </button>
        </div>
      </div>
      <div
        v-if="pausedByPlayer && canPause"
        class="absolute inset-0 z-20 flex flex-col items-center justify-center gap-3 rounded-xl bg-gray-950/85 p-4 text-center"
      >
        <h2 class="text-xl font-semibold">Paused</h2>
        <button
          type="button"
          class="rounded-lg bg-emerald-600 px-6 py-2 font-medium whitespace-nowrap text-white hover:bg-emerald-500"
          @click="setPausedByPlayer(false)"
        >
          Resume
        </button>
      </div>
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
