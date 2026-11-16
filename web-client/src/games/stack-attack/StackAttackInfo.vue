<script setup lang="ts">
import { computed } from "vue";

import { type StackAttackView, StackAttackWorkerStatus } from "@/proto/minigames/v2/stack_attack_pb";

import { workerColor } from "./worker-palette";

const props = defineProps<{ view: StackAttackView; seatNames: readonly string[]; status: string }>();

// Mirrors the rules' POINTS_PER_LINE (crates/games/stack-attack/src/rules/constants.rs).
const POINTS_PER_LINE = 100;

const STATUS_NOTES: Partial<Record<StackAttackWorkerStatus, string>> = {
  [StackAttackWorkerStatus.CRUSHED]: "crushed",
  [StackAttackWorkerStatus.LEFT]: "left",
};

const survived = computed(() => {
  const seconds = Math.floor((Number(props.view.ticksSurvived) * props.view.tickMs) / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
});

const stats = computed(() => [
  { label: "Score", value: String(props.view.score) },
  { label: "Level", value: String(props.view.level) },
  { label: "Lines", value: String(props.view.linesCleared) },
  { label: "Pushed", value: String(props.view.boxesPushed) },
  { label: "Time", value: survived.value },
]);

const workers = computed(() =>
  props.view.workers.map((worker) => ({
    slot: worker.slot,
    name: props.seatNames[worker.slot] ?? `Player ${worker.slot + 1}`,
    isYou: worker.slot === props.view.you,
    // A local player is often named "You" already: "You (you)" says it twice.
    showYouTag:
      worker.slot === props.view.you && props.seatNames[worker.slot]?.trim().toLowerCase() !== "you",
    alive: worker.status === StackAttackWorkerStatus.ALIVE,
    note: STATUS_NOTES[worker.status] ?? "",
    color: workerColor(worker.slot),
  })),
);
</script>

<template>
  <aside class="flex min-h-0 flex-col gap-2 rounded-lg bg-panel p-2 md:p-3">
    <p v-if="status" class="text-center text-sm text-gray-300">{{ status }}</p>
    <dl class="grid grid-cols-5 gap-1 text-center md:grid-cols-2 md:text-left">
      <div v-for="stat in stats" :key="stat.label" class="rounded bg-panel-raised px-1 py-1 md:px-2">
        <dt class="text-[0.65rem] uppercase tracking-wide text-gray-400">{{ stat.label }}</dt>
        <dd class="font-mono text-sm font-semibold text-emerald-300">{{ stat.value }}</dd>
      </div>
    </dl>
    <ul class="grid grid-cols-2 gap-1 md:grid-cols-1">
      <li
        v-for="worker in workers"
        :key="worker.slot"
        class="flex items-center gap-2 rounded px-2 py-1 text-sm"
        :class="[
          worker.isYou ? 'bg-panel-raised ring-1 ring-white/40' : '',
          worker.alive ? '' : 'opacity-50',
        ]"
      >
        <span class="size-3 shrink-0 rounded-full" :style="{ backgroundColor: worker.color }" />
        <span class="min-w-0 flex-1 truncate">
          {{ worker.name }}
          <span v-if="worker.showYouTag" class="text-xs text-gray-400"> (you)</span>
        </span>
        <span v-if="worker.note" class="text-xs text-rose-300">{{ worker.note }}</span>
      </li>
    </ul>
    <p class="text-center text-xs text-gray-400 md:text-left">
      Push crates to fill a row: a full row clears for {{ POINTS_PER_LINE }} points. Don't get crushed, and
      keep the stack below the red line.
    </p>
    <p class="text-center text-xs text-gray-400 md:text-left">
      Jump into a falling crate to smash it before it lands.
    </p>
  </aside>
</template>
