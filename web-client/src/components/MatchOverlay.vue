<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

import type { MatchController } from "@/match/match-controller";
import type { SeatPause } from "@/match/seat-pause";
import { OutcomeKind } from "@/proto/minigames/v2/match_pb";
import { seatLabels } from "@/util/players";

const props = defineProps<{ controller: MatchController }>();
const emit = defineEmits<{ leave: []; replay: []; again: [] }>();

const now = ref(performance.now());
const timer = setInterval(() => (now.value = performance.now()), 250);
onBeforeUnmount(() => clearInterval(timer));

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
const ended = computed(() => props.controller.ended.value);
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

async function voteDrop(slot: number): Promise<void> {
  await props.controller.source.voteDrop(slot).catch(() => undefined);
}
</script>

<template>
  <div
    v-if="pauses.length > 0 || countdown || ended"
    class="absolute inset-0 z-20 flex items-center justify-center bg-black/60 p-4"
  >
    <div class="w-full max-w-sm space-y-3 rounded-xl bg-panel p-5 text-center shadow-xl">
      <template v-if="ended">
        <h2 class="text-2xl font-bold">{{ headline }}</h2>
        <p v-if="aborted" class="text-sm text-gray-300">The server stopped this match. No replay is kept.</p>
        <ul v-if="result" class="space-y-1 text-sm">
          <li v-for="(score, slot) in result.scores" :key="slot" class="flex justify-between">
            <span>
              {{ seatName(slot) }}
              <span v-if="slot === mySlot" class="text-xs text-sky-300">(you)</span>
            </span>
            <span class="font-mono">{{ score }}</span>
          </li>
        </ul>
        <div class="flex flex-wrap justify-center gap-2 pt-2">
          <button class="rounded bg-emerald-600 px-4 py-2" @click="emit('again')">Play again</button>
          <button v-if="!aborted" class="rounded bg-panel-raised px-4 py-2" @click="emit('replay')">
            Save replay
          </button>
          <button class="rounded bg-panel-raised px-4 py-2" @click="emit('leave')">Leave</button>
        </div>
      </template>
      <template v-else-if="pauses.length > 0">
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
