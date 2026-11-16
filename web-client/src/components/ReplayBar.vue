<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";

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

const speeds = [25, 50, 100, 200, 400];
</script>

<template>
  <div class="flex flex-wrap items-center gap-3 rounded-xl bg-panel p-3 text-sm">
    <button class="rounded bg-panel-raised px-3 py-1" @click="playing ? replay.pause() : replay.play()">
      {{ playing ? "Pause" : "Play" }}
    </button>
    <button class="rounded bg-panel-raised px-3 py-1" @click="replay.step()">Step</button>
    <input
      type="range"
      class="min-w-32 flex-1"
      min="0"
      :max="replay.length"
      :value="position"
      @change="replay.seek(Number(($event.target as HTMLInputElement).value))"
    />
    <span class="font-mono text-xs text-gray-400">{{ position }}/{{ replay.length }}</span>
    <select
      class="rounded bg-panel-raised px-2 py-1"
      :value="speed"
      @change="replay.setSpeed(Number(($event.target as HTMLSelectElement).value))"
    >
      <option v-for="value in speeds" :key="value" :value="value">{{ value / 100 }}×</option>
    </select>
  </div>
</template>
