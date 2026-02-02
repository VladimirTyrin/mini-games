<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useGameStore } from "../../stores/game";
import { GameStatus, HintMode } from "../../proto/games/numbers_match_pb";

const FIELD_WIDTH = 9;

const gameStore = useGameStore();

const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = ref({ width: 0, height: 0 });
const selectedIndex = ref<number | null>(null);

const state = computed(() => gameStore.numbersMatchState);

const MIN_CELL_SIZE = 24;
const MAX_CELL_SIZE = 44;
const GAP_SIZE = 2;

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return 36;

  const padding = 16;
  const availableWidth = containerSize.value.width - padding * 2;
  const availableHeight = containerSize.value.height - 160;

  const totalGapsX = (FIELD_WIDTH - 1) * GAP_SIZE;
  const totalGapsY = (state.value.rowCount - 1) * GAP_SIZE;

  const cellByWidth = Math.floor((availableWidth - totalGapsX) / FIELD_WIDTH);
  const cellByHeight = Math.floor((availableHeight - totalGapsY) / state.value.rowCount);

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(MIN_CELL_SIZE, Math.min(MAX_CELL_SIZE, size));
});

const fontSize = computed(() => {
  const size = cellSize.value;
  if (size >= 44) return "text-xl";
  if (size >= 36) return "text-lg";
  return "text-base";
});

const gameInProgress = computed(() => {
  return state.value?.status === GameStatus.IN_PROGRESS;
});

const statusText = computed(() => {
  if (!state.value) return "";
  switch (state.value.status) {
    case GameStatus.WON:
      return "Victory!";
    case GameStatus.LOST:
      return "No moves left";
    default:
      return "";
  }
});

const hintsCount = computed(() => {
  if (!state.value) return 0;
  if (state.value.hintMode === HintMode.UNLIMITED) return "∞";
  if (state.value.hintMode === HintMode.DISABLED) return 0;
  return state.value.hintsRemaining ?? 0;
});

const hasActiveHint = computed(() => {
  return state.value?.currentHint != null;
});

const canUseHint = computed(() => {
  if (!state.value || !gameInProgress.value) return false;
  if (hasActiveHint.value) return false;
  if (state.value.hintMode === HintMode.DISABLED) return false;
  if (state.value.hintMode === HintMode.UNLIMITED) return true;
  return (state.value.hintsRemaining ?? 0) > 0;
});

const canRefill = computed(() => {
  if (!state.value || !gameInProgress.value) return false;
  return state.value.refillsRemaining > 0;
});

const hintIndices = computed(() => {
  if (!state.value?.currentHint) return new Set<number>();
  const hint = state.value.currentHint;
  if (hint.hint.case === "pair") {
    return new Set([hint.hint.value.firstIndex, hint.hint.value.secondIndex]);
  }
  return new Set<number>();
});

const showSuggestRefill = computed(() => {
  return state.value?.currentHint?.hint.case === "suggestRefill";
});

interface GridCell {
  index: number;
  row: number;
  col: number;
  value: number;
  removed: boolean;
  isEmpty: boolean;
  key: string;
}

const gridCells = computed((): GridCell[] => {
  if (!state.value) return [];

  return state.value.cells.map((cell, i) => ({
    index: i,
    row: Math.floor(i / FIELD_WIDTH),
    col: i % FIELD_WIDTH,
    value: cell.value,
    removed: cell.removed,
    isEmpty: cell.value === 0,
    key: `cell-${i}`,
  }));
});

const activeCellCount = computed(() => {
  if (!state.value) return 0;
  return state.value.cells.filter((c) => c.value > 0 && !c.removed).length;
});

function isActive(cell: GridCell): boolean {
  return cell.value > 0 && !cell.removed;
}

function handleCellClick(cell: GridCell): void {
  if (!gameInProgress.value || !isActive(cell)) return;

  if (selectedIndex.value === null) {
    selectedIndex.value = cell.index;
  } else if (selectedIndex.value === cell.index) {
    selectedIndex.value = null;
  } else {
    gameStore.sendNumbersMatchRemovePair(selectedIndex.value, cell.index);
    selectedIndex.value = null;
  }
}

function handleRefill(): void {
  if (canRefill.value) {
    gameStore.sendNumbersMatchRefill();
    selectedIndex.value = null;
  }
}

function handleHint(): void {
  if (canUseHint.value) {
    gameStore.sendNumbersMatchRequestHint();
    selectedIndex.value = null;
  }
}

function getCellClasses(cell: GridCell): Record<string, boolean> {
  const active = isActive(cell);
  const isSelected = selectedIndex.value === cell.index;
  const isHinted = hintIndices.value.has(cell.index);

  return {
    "cursor-pointer hover:bg-gray-600": active && gameInProgress.value,
    "ring-2 ring-yellow-400 bg-gray-600": isSelected,
    "ring-2 ring-green-400 bg-green-900/50 animate-pulse": isHinted && !isSelected,
    "bg-gray-700": active && !isSelected && !isHinted,
    "bg-gray-800/50": cell.removed && !cell.isEmpty,
    "bg-transparent": cell.isEmpty,
  };
}

function getCellTextClasses(cell: GridCell): string {
  if (cell.removed) return "text-gray-500";
  return "text-white";
}

function handleKeyDown(event: KeyboardEvent): void {
  if (event.key === "h" || event.key === "H") {
    handleHint();
  } else if (event.key === "f" || event.key === "F") {
    handleRefill();
  } else if (event.key === "Escape") {
    selectedIndex.value = null;
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
  >
    <!-- Top stats bar -->
    <div class="w-full flex justify-between items-center text-xs text-gray-400 py-2">
      <span>Cells: {{ activeCellCount }}</span>
      <span
        v-if="statusText"
        class="font-medium"
        :class="{
          'text-green-500': state?.status === GameStatus.WON,
          'text-red-500': state?.status === GameStatus.LOST,
        }"
      >
        {{ statusText }}
      </span>
      <span
        v-if="showSuggestRefill"
        class="text-amber-500 animate-pulse"
      >
        No pairs - refill!
      </span>
    </div>

    <!-- Game field -->
    <div v-if="state" class="flex-1 flex items-center justify-center py-2">
      <div
        class="grid bg-gray-800 p-1 rounded-lg"
        :style="{
          gridTemplateColumns: `repeat(${FIELD_WIDTH}, ${cellSize}px)`,
          gap: `${GAP_SIZE}px`,
        }"
      >
        <button
          v-for="cell in gridCells"
          :key="cell.key"
          class="rounded flex items-center justify-center font-medium transition-all"
          :class="[getCellClasses(cell), fontSize]"
          :style="{ width: `${cellSize}px`, height: `${cellSize}px` }"
          :disabled="!isActive(cell) || !gameInProgress"
          @click="handleCellClick(cell)"
        >
          <span v-if="cell.value > 0" :class="getCellTextClasses(cell)">
            {{ cell.value }}
          </span>
        </button>
      </div>
    </div>

    <!-- Bottom controls -->
    <div class="flex justify-center items-center gap-8 py-4">
      <!-- Refill button -->
      <button
        :disabled="!canRefill"
        class="relative w-14 h-14 rounded-full flex items-center justify-center transition-all text-2xl"
        :class="{
          'bg-blue-100 text-blue-600 hover:bg-blue-200 active:scale-95': canRefill,
          'bg-gray-100 text-gray-300 cursor-not-allowed': !canRefill,
        }"
        @click="handleRefill"
        title="Refill (F)"
      >
        <span>+</span>
        <span
          v-if="state"
          class="absolute -top-1 -right-1 w-5 h-5 rounded-full text-xs flex items-center justify-center font-medium"
          :class="{
            'bg-blue-500 text-white': canRefill,
            'bg-gray-300 text-gray-500': !canRefill,
          }"
        >
          {{ state.refillsRemaining }}
        </span>
      </button>

      <!-- Hint button -->
      <button
        :disabled="!canUseHint"
        class="relative w-14 h-14 rounded-full flex items-center justify-center transition-all text-2xl"
        :class="{
          'bg-amber-100 text-amber-600 hover:bg-amber-200 active:scale-95': canUseHint,
          'bg-gray-100 text-gray-300 cursor-not-allowed': !canUseHint,
        }"
        @click="handleHint"
        title="Hint (H)"
      >
        <span>💡</span>
        <span
          v-if="state && state.hintMode !== HintMode.DISABLED"
          class="absolute -top-1 -right-1 w-5 h-5 rounded-full text-xs flex items-center justify-center font-medium"
          :class="{
            'bg-amber-500 text-white': canUseHint,
            'bg-gray-300 text-gray-500': !canUseHint,
          }"
        >
          {{ hintsCount }}
        </span>
      </button>
    </div>
  </div>
</template>
