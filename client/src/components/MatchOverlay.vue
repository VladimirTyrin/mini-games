<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import { prefersTouch } from "@/input/prefers-touch";
import { useGameKeys } from "@/input/use-game-keys";
import type { MatchController } from "@/match/match-controller";
import type { SeatPause } from "@/match/seat-pause";
import { OutcomeKind } from "@/proto/minigames/v2/match_pb";
import { seatLabels } from "@/util/players";

const props = defineProps<{
  controller: MatchController;
  /** Why the match ended, from the game ("Crushed by a falling crate"). */
  endReason?: string | null;
}>();
const emit = defineEmits<{ leave: []; replay: []; again: [] }>();

const now = ref(performance.now());
const timer = setInterval(() => (now.value = performance.now()), 250);

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

// Enter or Space starts the next match once the result is up (keys held from the game's last
// moment do not count: repeats are ignored).
useGameKeys((key) => {
  if (!ended.value || resultFolded.value || (key !== "Enter" && key !== " ")) return false;
  emit("again");
  return true;
});
const showsKeyHint = !prefersTouch();

const pauses = computed(() => props.controller.pauses.value);
const aborted = computed(() => props.controller.aborted.value);
const secondsUntil = (at: number) => Math.max(0, Math.ceil((at - now.value) / 1000));
const canVoteDrop = (pause: SeatPause) =>
  props.controller.mySlot !== null && props.controller.mySlot !== pause.slot && pause.dropVoteAt <= now.value;
const countdown = computed(() => {
  const at = props.controller.resumeAt.value;
  return at && at > now.value ? Math.ceil((at - now.value) / 1000) : null;
});
const result = computed(() => props.controller.result.value);
const ended = computed(() => props.controller.ended.value && revealed.value);
const mySlot = computed(() => props.controller.mySlot);

const headline = computed(() => {
  const outcome = result.value;
  if (aborted.value) return "Match aborted";
  if (!outcome) return "Game over";
  switch (outcome.kind) {
    case OutcomeKind.WINNER:
      if (mySlot.value === null) return "Game over";
      return outcome.winnerSlots.includes(mySlot.value) ? "You win!" : "You lose";
    case OutcomeKind.DRAW:
      return "Draw";
    case OutcomeKind.VICTORY:
      return "Victory!";
    default:
      return "Game over";
  }
});

const labels = computed(() => seatLabels(props.controller.source.header));
const seatName = (slot: number) => labels.value[slot] ?? `Player ${slot + 1}`;

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
  <div v-if="ended" class="pointer-events-none absolute inset-x-0 bottom-0 z-20 flex justify-center p-3">
    <button
      v-if="resultFolded"
      class="pointer-events-auto rounded-full bg-panel/95 px-4 py-2 text-sm font-semibold shadow-lg ring-1 ring-white/10"
      @click="resultFolded = false"
    >
      {{ headline }} · show result ▴
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
            The server stopped this match. No replay is kept.
          </p>
        </div>
        <button
          class="-m-1 rounded-lg px-2 py-1 text-sm text-gray-400 hover:bg-white/10"
          aria-label="Hide the result to see the board"
          @click="resultFolded = true"
        >
          Hide ▾
        </button>
      </div>
      <ul v-if="result" class="space-y-1 text-sm">
        <li v-for="(score, slot) in result.scores" :key="slot" class="flex justify-between">
          <span>
            {{ seatName(slot) }}
            <span v-if="slot === mySlot" class="text-xs text-sky-300">(you)</span>
          </span>
          <span class="font-mono">{{ score }}</span>
        </li>
      </ul>
      <div class="flex flex-wrap gap-2">
        <button class="h-11 flex-1 rounded-xl bg-emerald-600 px-4 font-semibold" @click="emit('again')">
          Play again
          <kbd v-if="showsKeyHint" class="ml-1.5 rounded bg-white/20 px-1.5 py-0.5 text-xs font-normal"
            >Enter</kbd
          >
        </button>
        <button v-if="!aborted" class="h-11 rounded-xl bg-panel-raised px-4" @click="emit('replay')">
          Save replay
        </button>
        <button class="h-11 rounded-xl bg-panel-raised px-4" @click="emit('leave')">Leave</button>
      </div>
    </div>
  </div>
  <div
    v-else-if="pauses.length > 0 || countdown"
    class="absolute inset-0 z-20 flex items-center justify-center bg-black/60 p-4"
  >
    <div class="w-full max-w-sm space-y-3 rounded-xl bg-panel p-5 text-center shadow-xl">
      <template v-if="pauses.length > 0">
        <h2 class="text-xl font-semibold">Paused</h2>
        <div v-for="pause in pauses" :key="pause.slot" class="space-y-2">
          <p class="text-sm text-gray-300">
            {{ seatName(pause.slot) }} lost connection. Waiting up to
            {{ secondsUntil(pause.budgetEndsAt) }} s.
          </p>
          <button
            v-if="canVoteDrop(pause)"
            class="rounded bg-rose-700 px-4 py-2 text-sm"
            @click="voteDrop(pause.slot)"
          >
            Continue without {{ seatName(pause.slot) }}
          </button>
        </div>
      </template>
      <template v-else-if="countdown">
        <p class="text-5xl font-bold">{{ countdown }}</p>
      </template>
    </div>
  </div>
</template>
