<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useReplayStore } from "../../stores/replay";
import { useLobbyStore } from "../../stores/lobby";
import { useDeviceStore } from "../../stores/device";

const replayStore = useReplayStore();
const lobbyStore = useLobbyStore();
const deviceStore = useDeviceStore();

const isHost = computed(() => lobbyStore.isHost);
const isDisabled = computed(() => replayStore.hostOnlyControl && !isHost.value);

const speeds = [0.25, 0.5, 1, 2, 4];

const currentSpeedIndex = computed(() => {
  const idx = speeds.indexOf(replayStore.speed);
  return idx >= 0 ? idx : 2;
});

function decreaseSpeed(): void {
  if (isDisabled.value) return;
  const idx = currentSpeedIndex.value;
  if (idx > 0) {
    replayStore.setSpeed(speeds[idx - 1]!);
  }
}

function increaseSpeed(): void {
  if (isDisabled.value) return;
  const idx = currentSpeedIndex.value;
  if (idx < speeds.length - 1) {
    replayStore.setSpeed(speeds[idx + 1]!);
  }
}

function handleKeyDown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement | null;
  const isInputFocused =
    target !== null &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable);
  if (isInputFocused) return;

  if (event.code === "KeyS" && replayStore.replayData) {
    event.preventDefault();
    replayStore.saveReplay();
    return;
  }

  if (isDisabled.value) return;

  if (event.code === "Space") {
    event.preventDefault();
    replayStore.togglePause();
  } else if (event.code === "ArrowRight" && replayStore.isPaused) {
    event.preventDefault();
    replayStore.stepForward();
  } else if (event.code === "KeyR") {
    event.preventDefault();
    replayStore.restart();
  } else if (event.code === "Minus" || event.code === "NumpadSubtract") {
    event.preventDefault();
    decreaseSpeed();
  } else if (event.code === "Equal" || event.code === "NumpadAdd") {
    event.preventDefault();
    increaseSpeed();
  }
}

onMounted(() => {
  window.addEventListener("keydown", handleKeyDown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
});
</script>

<template>
  <div class="bg-gray-800 border border-gray-700 rounded-lg p-3 mt-2">
    <div v-if="isDisabled" class="text-center text-gray-400 text-sm mb-2">
      Host controls playback
    </div>

    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2">
        <button
          :disabled="isDisabled"
          class="px-3 py-1.5 rounded text-sm font-medium transition-colors"
          :class="isDisabled
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-blue-600 hover:bg-blue-700 text-white'"
          @click="replayStore.togglePause()"
        >
          <template v-if="replayStore.isPaused">
            Play<template v-if="!deviceStore.isTouchDevice"> (Space)</template>
          </template>
          <template v-else>
            Pause<template v-if="!deviceStore.isTouchDevice"> (Space)</template>
          </template>
        </button>

        <button
          :disabled="isDisabled || !replayStore.isPaused"
          class="px-2 py-1.5 rounded text-sm font-medium transition-colors"
          :class="isDisabled || !replayStore.isPaused
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gray-600 hover:bg-gray-500 text-white'"
          @click="replayStore.stepForward()"
        >
          Step<template v-if="!deviceStore.isTouchDevice"> (&#8594;)</template>
        </button>

        <button
          :disabled="isDisabled"
          class="px-2 py-1.5 rounded text-sm font-medium transition-colors"
          :class="isDisabled
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gray-600 hover:bg-gray-500 text-white'"
          @click="replayStore.restart()"
        >
          Restart<template v-if="!deviceStore.isTouchDevice"> (R)</template>
        </button>

        <button
          :disabled="!replayStore.replayData"
          class="px-2 py-1.5 rounded text-sm font-medium transition-colors"
          :class="!replayStore.replayData
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-emerald-600 hover:bg-emerald-700 text-white'"
          @click="replayStore.saveReplay()"
        >
          Save<template v-if="!deviceStore.isTouchDevice"> (S)</template>
        </button>
      </div>

      <div class="flex items-center gap-1">
        <button
          :disabled="isDisabled || currentSpeedIndex <= 0"
          class="px-1.5 py-1 rounded text-sm font-medium transition-colors"
          :class="isDisabled || currentSpeedIndex <= 0
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gray-600 hover:bg-gray-500 text-white'"
          @click="decreaseSpeed"
        >
          -
        </button>
        <span class="text-sm text-gray-300 w-12 text-center">{{ replayStore.speed }}x</span>
        <button
          :disabled="isDisabled || currentSpeedIndex >= speeds.length - 1"
          class="px-1.5 py-1 rounded text-sm font-medium transition-colors"
          :class="isDisabled || currentSpeedIndex >= speeds.length - 1
            ? 'bg-gray-700 text-gray-500 cursor-not-allowed'
            : 'bg-gray-600 hover:bg-gray-500 text-white'"
          @click="increaseSpeed"
        >
          +
        </button>
      </div>
    </div>

    <div class="mt-2">
      <div class="flex items-center gap-2">
        <div class="flex-1 bg-gray-700 rounded-full h-2 overflow-hidden">
          <div
            class="h-full bg-blue-500 transition-all duration-200"
            :style="{ width: `${replayStore.progress * 100}%` }"
          />
        </div>
        <span class="text-xs text-gray-400 whitespace-nowrap">
          {{ replayStore.currentTick }} / {{ replayStore.totalTicks }}
        </span>
      </div>
    </div>
  </div>
</template>
