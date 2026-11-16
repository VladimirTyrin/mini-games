<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

import type { MatchController } from "@/match/match-controller";
import { OutcomeKind } from "@/proto/minigames/v2/match_pb";

const props = defineProps<{ controller: MatchController }>();
const emit = defineEmits<{ leave: []; replay: []; again: [] }>();

const now = ref(performance.now());
const timer = setInterval(() => (now.value = performance.now()), 250);
onBeforeUnmount(() => clearInterval(timer));

const pause = computed(() => props.controller.pause.value);
const countdown = computed(() => {
  const at = props.controller.resumeAt.value;
  return at && at > now.value ? Math.ceil((at - now.value) / 1000) : null;
});
const result = computed(() => props.controller.result.value);
const ended = computed(() => props.controller.ended.value);
const mySlot = computed(() => props.controller.mySlot);

const headline = computed(() => {
  const outcome = result.value;
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

const seatName = (slot: number) =>
  props.controller.source.header.seats[slot]?.player?.displayName ?? `Player ${slot + 1}`;

async function voteDrop(): Promise<void> {
  if (pause.value) await props.controller.source.voteDrop(pause.value.slot).catch(() => undefined);
}
</script>

<template>
  <div
    v-if="pause || countdown || ended"
    class="absolute inset-0 z-20 flex items-center justify-center bg-black/60 p-4"
  >
    <div class="w-full max-w-sm space-y-3 rounded-xl bg-panel p-5 text-center shadow-xl">
      <template v-if="ended">
        <h2 class="text-2xl font-bold">{{ headline }}</h2>
        <ul v-if="result" class="space-y-1 text-sm">
          <li v-for="(score, slot) in result.scores" :key="slot" class="flex justify-between">
            <span>{{ seatName(slot) }}</span>
            <span class="font-mono">{{ score }}</span>
          </li>
        </ul>
        <div class="flex flex-wrap justify-center gap-2 pt-2">
          <button class="rounded bg-emerald-600 px-4 py-2" @click="emit('again')">Play again</button>
          <button class="rounded bg-panel-raised px-4 py-2" @click="emit('replay')">Save replay</button>
          <button class="rounded bg-panel-raised px-4 py-2" @click="emit('leave')">Leave</button>
        </div>
      </template>
      <template v-else-if="pause">
        <h2 class="text-xl font-semibold">Paused</h2>
        <p class="text-sm text-gray-300">
          {{ seatName(pause.slot) }} lost connection. Waiting up to
          {{ Math.ceil(pause.budgetLeftMs / 1000) }} s.
        </p>
        <button
          v-if="controller.mySlot !== null && controller.mySlot !== pause.slot"
          class="rounded bg-rose-700 px-4 py-2 text-sm"
          @click="voteDrop"
        >
          Continue without them
        </button>
      </template>
      <template v-else-if="countdown">
        <p class="text-5xl font-bold">{{ countdown }}</p>
      </template>
    </div>
  </div>
</template>
