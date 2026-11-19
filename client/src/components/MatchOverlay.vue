<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { pluginBySettings } from "@/games";
import { t } from "@/i18n";
import { prefersTouch } from "@/input/prefers-touch";
import { useGameKeys } from "@/input/use-game-keys";
import type { MatchController } from "@/match/match-controller";
import { hasCounted } from "@/match/match-series";
import type { SeatPause } from "@/match/seat-pause";
import type { MatchSeries } from "@/proto/minigames/v2/lobby_pb";
import { OutcomeKind } from "@/proto/minigames/v2/match_pb";
import { useConnectionStore } from "@/stores/connection";
import { useLobbyStore } from "@/stores/lobby";
import { sameIdentity } from "@/util/players";
import { gameCatalog } from "@/wasm/core";
import { matchSeatNames } from "@/games/seat-names";

const props = defineProps<{
  controller: MatchController;
  /** Why the match ended, from the game ("Crushed by a falling crate"). */
  endReason?: string | null;
  /** The score of the matches played again at this table, this one included. */
  series?: MatchSeries | null;
}>();
const emit = defineEmits<{ leave: []; replay: []; again: [] }>();

const now = ref(performance.now());
// Ticks only while a countdown shows (a seat away, the resume countdown): otherwise it would
// re-render the overlay four times a second for the whole match.
const timer = setInterval(() => {
  if (props.controller.pauses.value.length > 0 || props.controller.resumeAt.value !== null)
    now.value = performance.now();
}, 250);

// The result waits until the ending (a crush, the winning move) has been seen.
const revealed = ref(false);
let revealTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => props.controller.resultRevealAt.value,
  (revealAt) => {
    if (revealTimer !== null) clearTimeout(revealTimer);
    revealTimer = null;
    revealed.value = revealAt !== null && revealAt <= performance.now();
    if (revealAt !== null && !revealed.value)
      revealTimer = setTimeout(() => (revealed.value = true), revealAt - performance.now());
  },
  { immediate: true },
);
onBeforeUnmount(() => {
  clearInterval(timer);
  if (revealTimer !== null) clearTimeout(revealTimer);
});

const isReplay = props.controller.source.kind === "replay";

// Enter starts the next match once the result is up (keys held from the game's last moment do
// not count: repeats are ignored). Not Space, in any game: Stack Attack jumps with it and a replay
// plays with it, so one pressed as the game ends would start a new match by accident, and a key
// that plays again in some games only would be one more thing to remember.
useGameKeys((key) => {
  if (!ended.value || resultFolded.value || key !== "Enter" || !canPlayAgain.value) return false;
  emit("again");
  return true;
});
const showsKeyHint = !prefersTouch();

const pauses = computed(() => props.controller.pauses.value);
const aborted = computed(() => props.controller.aborted.value);
const secondsUntil = (at: number) => Math.max(0, Math.ceil((at - now.value) / 1000));
/** A player still in the match, other than the one waited for, has a say on going on without them. */
const mayVoteOn = (pause: SeatPause) =>
  props.controller.mySlot !== null &&
  !props.controller.droppedSlots.value.has(props.controller.mySlot) &&
  props.controller.mySlot !== pause.slot;
const canVoteDrop = (pause: SeatPause) => mayVoteOn(pause) && pause.dropVoteAt <= now.value;
/** "2:45" */
const minutesUntil = (at: number) => {
  const seconds = secondsUntil(at);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
};
const countdown = computed(() => {
  const at = props.controller.resumeAt.value;
  return at && at > now.value ? Math.ceil((at - now.value) / 1000) : null;
});
const result = computed(() => props.controller.result.value);
const ended = computed(() => props.controller.ended.value && revealed.value);
const mySlot = computed(() => props.controller.mySlot);
const lobby = useLobbyStore();
const connection = useConnectionStore();
/** A table of bots online, watched by its lobby's host: the server takes the host's vote then. */
const hostsBotsOnlyMatch = computed(
  () =>
    props.controller.source.kind === "online" &&
    props.controller.source.header.seats.every((seat) => seat.botKind !== undefined) &&
    sameIdentity(lobby.current?.host, connection.identity),
);
/** Watching bots on this device: nobody else to ask, the same table plays again. */
const watchesLocally = props.controller.source.kind === "local" && props.controller.mySlot === null;
/**
 * A replay can always be watched again. A match is played again by its players, or by whoever
 * watches a table of bots (the lobby's host online), and only after a real end: an aborted one
 * left nothing to repeat.
 */
const canPlayAgain = computed(
  () => isReplay || ((mySlot.value !== null || hostsBotsOnlyMatch.value || watchesLocally) && !aborted.value),
);

const headline = computed(() => {
  const outcome = result.value;
  const texts = t.match.result;
  if (aborted.value) return isReplay ? texts.replayEnds : texts.aborted;
  if (!outcome) return texts.gameOver;
  switch (outcome.kind) {
    case OutcomeKind.WINNER:
      if (mySlot.value === null) return texts.gameOver;
      return outcome.winnerSlots.includes(mySlot.value) ? texts.youWin : texts.youLose;
    case OutcomeKind.DRAW:
      return texts.draw;
    case OutcomeKind.VICTORY:
      return texts.victory;
    default:
      return texts.gameOver;
  }
});

/** Wins of the series, once a match of it counted (never alone, never in a cooperative game). */
const shownSeries = computed(() => (hasCounted(props.series) ? props.series : null));
/** Points of the game's own are worth a column beside the wins; a mere 1 for the winner is not. */
const showsMatchScores = computed(() => {
  if (!shownSeries.value) return true;
  const gameId = pluginBySettings(props.controller.source.header.settings?.game.case)?.id;
  return gameCatalog().find((game) => game.id === gameId)?.scores !== "winFlag";
});

const labels = computed(() => matchSeatNames(props.controller.source.header));
const seatName = (slot: number) => labels.value[slot] ?? t.match.player(slot + 1);

const resultFolded = ref(false);
watch(
  () => props.controller,
  () => (resultFolded.value = false),
);

async function voteDrop(slot: number): Promise<void> {
  await props.controller.source.voteDrop(slot).catch(() => undefined);
}
</script>

<template>
  <!-- The result docks at the bottom without dimming the board, so the final position stays
       visible; it can be folded away entirely. -->
  <div
    v-if="ended"
    data-testid="match-result"
    class="pointer-events-none absolute inset-x-0 bottom-0 z-20 flex justify-center p-3"
  >
    <button
      v-if="resultFolded"
      class="pointer-events-auto rounded-full bg-panel/95 px-4 py-2 text-sm font-semibold shadow-lg ring-1 ring-white/10"
      @click="resultFolded = false"
    >
      {{ `${headline} · ${t.match.result.show} ▴` }}
    </button>
    <div
      v-else
      class="pointer-events-auto w-full max-w-md space-y-3 rounded-2xl bg-panel/95 p-4 shadow-xl ring-1 ring-white/10 backdrop-blur"
    >
      <div class="flex items-start gap-3">
        <div class="min-w-0 flex-1">
          <h2 class="text-xl font-bold">{{ headline }}</h2>
          <p v-if="endReason && !aborted" class="text-sm text-gray-300">{{ endReason }}</p>
          <p v-if="aborted" class="text-sm text-gray-300">
            {{ isReplay ? t.match.result.replaySavedBeforeEnd : t.match.result.stoppedByServer }}
          </p>
        </div>
        <button
          class="-m-1 rounded-lg px-2 py-1 text-sm text-gray-400 hover:bg-white/10"
          :aria-label="t.match.result.hideLabel"
          @click="resultFolded = true"
        >
          {{ `${t.match.result.hide} ▾` }}
        </button>
      </div>
      <ul v-if="result" class="space-y-1 text-sm">
        <li v-if="shownSeries" class="flex gap-4 text-xs text-gray-400">
          <span class="flex-1" />
          <span v-if="showsMatchScores">{{ t.match.result.scoreColumn }}</span>
          <span class="min-w-8 text-right">{{ t.match.result.winsColumn }}</span>
        </li>
        <li v-for="(score, slot) in result.scores" :key="slot" class="flex gap-4">
          <span class="min-w-0 flex-1 break-words">
            {{ seatName(slot) }}
            <span v-if="slot === mySlot && seatName(slot) !== t.common.youName" class="text-xs text-sky-300"
              >({{ t.common.you }})</span
            >
          </span>
          <span v-if="showsMatchScores" class="font-mono">{{ score }}</span>
          <span v-if="shownSeries" class="min-w-8 text-right font-mono font-semibold">{{
            shownSeries.wins[slot] ?? 0
          }}</span>
        </li>
        <li v-if="shownSeries && shownSeries.draws > 0" class="text-xs text-gray-400">
          {{ t.match.series.draws(shownSeries.draws) }}
        </li>
      </ul>
      <!-- No wrapping inside a button: on a narrow phone the row wraps instead. -->
      <div class="flex flex-wrap gap-2 whitespace-nowrap">
        <button
          v-if="canPlayAgain"
          class="min-h-11 flex-1 rounded-xl bg-emerald-600 px-4 py-2 font-semibold"
          @click="emit('again')"
        >
          {{ isReplay ? t.match.result.watchAgain : t.match.playAgain }}
          <kbd v-if="showsKeyHint" class="ml-1.5 rounded bg-white/20 px-1.5 py-0.5 text-xs font-normal">{{
            t.match.result.enterKey
          }}</kbd>
        </button>
        <button
          v-if="!aborted && !isReplay"
          class="min-h-11 rounded-xl bg-panel-raised px-4 py-2"
          @click="emit('replay')"
        >
          {{ t.match.result.saveReplay }}
        </button>
        <button class="min-h-11 rounded-xl bg-panel-raised px-4 py-2" @click="emit('leave')">
          {{ t.common.leave }}
        </button>
      </div>
    </div>
  </div>
  <div
    v-else-if="pauses.length > 0 || countdown"
    class="absolute inset-0 z-20 flex items-center justify-center bg-black/60 p-4"
  >
    <div class="w-full max-w-sm space-y-3 rounded-xl bg-panel p-5 text-center shadow-xl">
      <template v-if="pauses.length > 0">
        <h2 class="text-xl font-semibold">{{ t.match.paused }}</h2>
        <div v-for="pause in pauses" :key="pause.slot" class="space-y-2">
          <p v-if="pause.budgetEndsAt === null" class="text-sm text-gray-300">
            {{ t.match.seatPause.theirTurn(seatName(pause.slot)) }}
          </p>
          <p v-else class="text-sm text-gray-300">
            {{ t.match.seatPause.waitingUpTo(seatName(pause.slot), secondsUntil(pause.budgetEndsAt)) }}
          </p>
          <p
            v-if="pause.budgetEndsAt === null && mayVoteOn(pause) && !canVoteDrop(pause)"
            class="text-xs text-gray-400"
          >
            {{ t.match.seatPause.canGoOnIn(minutesUntil(pause.dropVoteAt)) }}
          </p>
          <button
            v-if="canVoteDrop(pause)"
            class="rounded bg-rose-700 px-4 py-2 text-sm"
            @click="voteDrop(pause.slot)"
          >
            {{ t.match.seatPause.continueWithout(seatName(pause.slot)) }}
          </button>
        </div>
      </template>
      <template v-else-if="countdown">
        <p class="text-5xl font-bold">{{ countdown }}</p>
      </template>
    </div>
  </div>
</template>
