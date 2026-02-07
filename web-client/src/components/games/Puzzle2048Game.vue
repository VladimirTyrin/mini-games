<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useGameStore, Puzzle2048Direction } from "../../stores/game";
import { Puzzle2048GameStatus } from "../../proto/games/puzzle2048_pb";

const gameStore = useGameStore();

const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = ref({ width: 0, height: 0 });

const state = computed(() => gameStore.puzzle2048State);

const fieldWidth = computed(() => state.value?.fieldWidth ?? 4);
const fieldHeight = computed(() => state.value?.fieldHeight ?? 4);

const GAP_SIZE = 8;

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return 80;

  const padding = 16;
  const availableWidth = containerSize.value.width - padding * 2;
  const availableHeight = containerSize.value.height - 120;

  const totalGapsX = (fieldWidth.value - 1) * GAP_SIZE;
  const totalGapsY = (fieldHeight.value - 1) * GAP_SIZE;

  const cellByWidth = Math.floor((availableWidth - totalGapsX) / fieldWidth.value);
  const cellByHeight = Math.floor((availableHeight - totalGapsY) / fieldHeight.value);

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(40, Math.min(100, size));
});

const fontSize = computed(() => {
  const size = cellSize.value;
  if (size >= 80) return "text-2xl";
  if (size >= 60) return "text-xl";
  if (size >= 45) return "text-lg";
  return "text-base";
});

const gameInProgress = computed(() => {
  return state.value?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_IN_PROGRESS;
});

const statusText = computed(() => {
  if (!state.value) return "";
  switch (state.value.status) {
    case Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_WON:
      return "You Win!";
    case Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_LOST:
      return "Game Over";
    default:
      return "";
  }
});

function tileColor(value: number): string {
  const colors: Record<number, string> = {
    0: "bg-gray-700",
    2: "bg-[#eee4da] text-gray-800",
    4: "bg-[#ede0c8] text-gray-800",
    8: "bg-[#f2b179] text-white",
    16: "bg-[#f59563] text-white",
    32: "bg-[#f67c5f] text-white",
    64: "bg-[#f65e3b] text-white",
    128: "bg-[#edcf72] text-white",
    256: "bg-[#edcc61] text-white",
    512: "bg-[#edc850] text-white",
    1024: "bg-[#edc53f] text-white",
    2048: "bg-[#edc22e] text-white",
    4096: "bg-[#3c3a32] text-white",
    8192: "bg-[#3c3a32] text-white",
  };
  return colors[value] ?? "bg-[#3c3a32] text-white";
}

let touchStartX = 0;
let touchStartY = 0;

function handleTouchStart(event: TouchEvent): void {
  if (!gameInProgress.value) return;
  const touch = event.touches[0];
  if (!touch) return;
  touchStartX = touch.clientX;
  touchStartY = touch.clientY;
}

function handleTouchEnd(event: TouchEvent): void {
  if (!gameInProgress.value) return;
  const touch = event.changedTouches[0];
  if (!touch) return;
  const dx = touch.clientX - touchStartX;
  const dy = touch.clientY - touchStartY;

  const minSwipe = 30;
  if (Math.abs(dx) < minSwipe && Math.abs(dy) < minSwipe) return;

  event.preventDefault();

  if (Math.abs(dx) > Math.abs(dy)) {
    if (dx > 0) {
      gameStore.sendPuzzle2048Move(Puzzle2048Direction.PUZZLE_2048_DIRECTION_RIGHT);
    } else {
      gameStore.sendPuzzle2048Move(Puzzle2048Direction.PUZZLE_2048_DIRECTION_LEFT);
    }
  } else {
    if (dy > 0) {
      gameStore.sendPuzzle2048Move(Puzzle2048Direction.PUZZLE_2048_DIRECTION_DOWN);
    } else {
      gameStore.sendPuzzle2048Move(Puzzle2048Direction.PUZZLE_2048_DIRECTION_UP);
    }
  }
}

function handleKeyDown(event: KeyboardEvent): void {
  if (!gameInProgress.value) return;

  let direction: Puzzle2048Direction | null = null;

  switch (event.key) {
    case "ArrowUp":
    case "w":
    case "W":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_UP;
      break;
    case "ArrowDown":
    case "s":
    case "S":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_DOWN;
      break;
    case "ArrowLeft":
    case "a":
    case "A":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_LEFT;
      break;
    case "ArrowRight":
    case "d":
    case "D":
      direction = Puzzle2048Direction.PUZZLE_2048_DIRECTION_RIGHT;
      break;
  }

  if (direction !== null) {
    event.preventDefault();
    gameStore.sendPuzzle2048Move(direction);
  }
}

let resizeObserver: ResizeObserver | null = null;

function updateContainerSize(): void {
  if (containerRef.value) {
    containerSize.value = {
      width: containerRef.value.clientWidth,
      height: window.innerHeight - 80,
    };
  }
}

onMounted(() => {
  updateContainerSize();
  resizeObserver = new ResizeObserver(() => {
    updateContainerSize();
  });
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value);
  }
  window.addEventListener("keydown", handleKeyDown);
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
  window.removeEventListener("keydown", handleKeyDown);
});
</script>

<template>
  <div
    ref="containerRef"
    class="flex flex-col items-center w-full max-w-lg mx-auto px-2"
    @touchstart="handleTouchStart"
    @touchend="handleTouchEnd"
  >
    <div class="w-full flex justify-between items-center text-sm text-gray-400 py-3">
      <span class="text-lg font-bold text-white">Score: {{ state?.score ?? 0 }}</span>
      <span
        v-if="statusText"
        class="font-medium text-lg"
        :class="{
          'text-green-500': state?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_WON,
          'text-red-500': state?.status === Puzzle2048GameStatus.PUZZLE_2048_GAME_STATUS_LOST,
        }"
      >
        {{ statusText }}
      </span>
      <span class="text-sm">Target: {{ state?.targetValue ?? 2048 }}</span>
    </div>

    <div v-if="state" class="flex-1 flex items-center justify-center py-2">
      <div
        class="grid bg-gray-800 p-2 rounded-lg"
        :style="{
          gridTemplateColumns: `repeat(${fieldWidth}, ${cellSize}px)`,
          gap: `${GAP_SIZE}px`,
        }"
      >
        <div
          v-for="(cell, i) in state.cells"
          :key="i"
          class="rounded-md flex items-center justify-center font-bold transition-all"
          :class="[tileColor(cell), fontSize]"
          :style="{ width: `${cellSize}px`, height: `${cellSize}px` }"
        >
          <span v-if="cell > 0">{{ cell }}</span>
        </div>
      </div>
    </div>

    <div class="text-sm text-gray-500 py-2">
      Use arrow keys or WASD to move tiles
    </div>
  </div>
</template>
