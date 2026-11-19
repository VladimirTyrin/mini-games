<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

import { prefersTouch } from "@/input/prefers-touch";
import { useGameKeys } from "@/input/use-game-keys";
import { t } from "@/i18n";
import type { ReplayMatchSource } from "@/match/replay-match-source";

const props = defineProps<{ replay: ReplayMatchSource }>();
const position = ref(props.replay.current);
const playing = ref(props.replay.isPlaying);
const speed = ref(props.replay.speed);
const off = props.replay.onProgress(() => {
  position.value = props.replay.current;
  playing.value = props.replay.isPlaying;
  speed.value = props.replay.speed;
});
onBeforeUnmount(off);

const SPEEDS = [25, 50, 100, 200, 400] as const;
const speedIndex = computed(() => SPEEDS.findIndex((value) => value === speed.value));
const showsKeys = !prefersTouch();

function changeSpeed(by: number): void {
  const next = SPEEDS[Math.min(SPEEDS.length - 1, Math.max(0, speedIndex.value + by))];
  if (next !== undefined) props.replay.setSpeed(next);
}

function togglePlay(): void {
  if (playing.value) props.replay.pause();
  else props.replay.play();
}

/** "1:05" for real-time games; turn-based ones count moves. */
function moment(ticks: number): string {
  if (props.replay.isTurnBased) return `${ticks}`;
  const seconds = Math.floor((ticks * props.replay.gameTickMs) / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

// The v1 replay keys: Space plays or pauses, arrows step, R restarts, -/+ change the speed.
useGameKeys(
  (key) => {
    switch (key) {
      case " ":
        togglePlay();
        return true;
      case "ArrowRight":
        props.replay.step();
        return true;
      case "ArrowLeft":
        props.replay.stepBack();
        return true;
      case "r":
      case "R":
        props.replay.restart();
        return true;
      case "-":
        changeSpeed(-1);
        return true;
      case "+":
      case "=":
        changeSpeed(1);
        return true;
      default:
        return false;
    }
  },
  { allowRepeat: true },
);

const BUTTON =
  "flex size-11 shrink-0 items-center justify-center rounded-lg bg-panel-raised text-lg hover:bg-white/10";
</script>

<template>
  <div class="flex flex-col gap-2 rounded-xl bg-panel p-3 text-sm">
    <div class="flex items-center gap-3">
      <span class="w-12 text-right font-mono text-xs text-gray-400">{{ moment(position) }}</span>
      <input
        type="range"
        class="h-2 min-w-0 flex-1 accent-emerald-500"
        :aria-label="t.match.replay.position"
        min="0"
        :max="replay.length"
        :value="position"
        @input="replay.seek(Number(($event.target as HTMLInputElement).value))"
      />
      <span class="w-12 font-mono text-xs text-gray-400">{{ moment(replay.length) }}</span>
    </div>
    <div class="flex flex-wrap items-center justify-center gap-2">
      <button
        :class="BUTTON"
        :aria-label="t.match.replay.restart"
        :title="showsKeys ? `${t.match.replay.restart} (R)` : undefined"
        @click="replay.restart()"
      >
        {{ "⏮" }}
      </button>
      <button
        :class="BUTTON"
        :aria-label="t.match.replay.stepBack"
        :title="showsKeys ? `${t.match.replay.stepBack} (←)` : undefined"
        @click="replay.stepBack()"
      >
        {{ "◀︎" }}
      </button>
      <button
        class="flex h-11 min-w-24 items-center justify-center gap-1.5 rounded-lg bg-emerald-600 px-4 font-semibold"
        :aria-label="playing ? t.match.replay.pause : t.match.replay.play"
        @click="togglePlay"
      >
        {{ playing ? `❚❚ ${t.match.replay.pause}` : `▶ ${t.match.replay.play}` }}
        <kbd v-if="showsKeys" class="rounded bg-white/20 px-1 text-xs font-normal">{{
          t.match.spaceKey
        }}</kbd>
      </button>
      <button
        :class="BUTTON"
        :aria-label="t.match.replay.stepForward"
        :title="showsKeys ? `${t.match.replay.step} (→)` : undefined"
        @click="replay.step()"
      >
        {{ "▶︎" }}
      </button>
      <div role="radiogroup" :aria-label="t.match.replay.speed" class="flex rounded-lg bg-panel-raised p-0.5">
        <button
          v-for="value in SPEEDS"
          :key="value"
          type="button"
          role="radio"
          :aria-checked="value === speed"
          class="h-10 rounded-md px-2 font-mono text-xs"
          :class="value === speed ? 'bg-emerald-600 text-white' : 'text-gray-300 hover:bg-white/10'"
          @click="replay.setSpeed(value)"
        >
          {{ value / 100 }}×
        </button>
      </div>
    </div>
    <p v-if="showsKeys" class="text-center text-xs text-gray-500">
      {{ t.match.replay.keysHint }}
    </p>
  </div>
</template>
