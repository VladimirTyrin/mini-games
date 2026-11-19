<script setup lang="ts">
import { computed } from "vue";

import { t } from "@/i18n";
import { type StackAttackView, StackAttackWorkerStatus } from "@/proto/minigames/v2/stack_attack_pb";

import { POINTS_PER_LINE } from "./points-per-line";
import { workerColor } from "./worker-palette";

const props = defineProps<{ view: StackAttackView; seatNames: readonly string[]; status: string }>();

function statusNoteOf(status: StackAttackWorkerStatus): string {
  switch (status) {
    case StackAttackWorkerStatus.CRUSHED:
      return t.games.stackAttack.info.crushed;
    case StackAttackWorkerStatus.LEFT:
      return t.games.stackAttack.info.left;
    default:
      return "";
  }
}

const survived = computed(() => {
  const seconds = Math.floor((Number(props.view.ticksSurvived) * props.view.tickMs) / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
});

const stats = computed(() => [
  { label: t.games.stackAttack.info.score, value: String(props.view.score) },
  { label: t.games.stackAttack.info.level, value: String(props.view.level) },
  { label: t.games.stackAttack.info.lines, value: String(props.view.linesCleared) },
  // The one a 320 px phone gives up, for five columns would not fit their labels.
  { label: t.games.stackAttack.info.pushed, value: String(props.view.boxesPushed), secondary: true },
  { label: t.games.stackAttack.info.time, value: survived.value },
]);

const workers = computed(() =>
  props.view.workers.map((worker) => ({
    slot: worker.slot,
    name: props.seatNames[worker.slot] ?? t.match.player(worker.slot + 1),
    isYou: worker.slot === props.view.you,
    // A local player is often named "You" already: "You (you)" says it twice.
    showYouTag:
      worker.slot === props.view.you &&
      props.seatNames[worker.slot]?.trim().toLowerCase() !== t.common.youName.toLowerCase(),
    alive: worker.status === StackAttackWorkerStatus.ALIVE,
    note: statusNoteOf(worker.status),
    color: workerColor(worker.slot),
  })),
);
</script>

<template>
  <aside class="flex min-h-0 flex-col gap-2 rounded-lg bg-panel p-2 md:p-3">
    <p v-if="status" class="text-center text-sm text-gray-300">{{ status }}</p>
    <dl
      class="grid grid-cols-5 gap-1 text-center max-[359px]:grid-cols-4 landscape:grid-cols-2 landscape:text-left"
    >
      <div
        v-for="stat in stats"
        :key="stat.label"
        class="rounded bg-panel-raised px-1 py-1 landscape:px-2"
        :class="{ 'max-[359px]:hidden': stat.secondary }"
      >
        <dt class="text-[0.65rem] uppercase tracking-wide text-gray-400">{{ stat.label }}</dt>
        <dd class="font-mono text-sm font-semibold text-emerald-300">{{ stat.value }}</dd>
      </div>
    </dl>
    <ul class="grid grid-cols-2 gap-1 landscape:grid-cols-1">
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
          <span v-if="worker.showYouTag" class="text-xs text-gray-400">
            {{ t.games.stackAttack.info.you }}</span
          >
        </span>
        <span v-if="worker.note" class="text-xs text-rose-300">{{ worker.note }}</span>
      </li>
    </ul>
    <p class="text-center text-xs text-gray-400 landscape:text-left">
      {{ t.games.stackAttack.info.howToScore(POINTS_PER_LINE) }}
    </p>
    <p class="text-center text-xs text-gray-400 landscape:text-left">
      {{ t.games.stackAttack.info.smash }}
    </p>
  </aside>
</template>
