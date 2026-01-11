<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useGameStore } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";
import { MarkType, GameStatus } from "../../proto/games/tictactoe_pb";
import type { WinningLine } from "../../proto/games/tictactoe_pb";

const props = defineProps<{
  showWinningLine?: boolean;
}>();

const gameStore = useGameStore();
const connectionStore = useConnectionStore();

const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = ref({ width: 0, height: 0 });

const state = computed(() => gameStore.tictactoeState);

const winningLine = computed((): WinningLine | null => {
  if (!props.showWinningLine) return null;
  const gameInfo = gameStore.gameOver?.gameInfo;
  if (gameInfo?.case !== "tictactoeInfo") return null;
  return gameInfo.value.winningLine ?? null;
});

const BASE_CELL_SIZE = 48;
const MIN_CELL_SIZE = 16;
const MAX_CELL_SIZE = 64;
const GAP_SIZE = 4;

const isMobileLayout = computed(() => {
  return containerSize.value.width < 1024;
});

const cellSize = computed(() => {
  if (!state.value || containerSize.value.width === 0) return BASE_CELL_SIZE;

  const padding = isMobileLayout.value ? 0 : 48;
  const sidebarWidth = isMobileLayout.value ? 0 : 288;
  const availableWidth = containerSize.value.width - sidebarWidth - padding;
  const availableHeight = containerSize.value.height - (isMobileLayout.value ? 200 : 100);

  const totalGapsX = (state.value.fieldWidth - 1) * GAP_SIZE;
  const totalGapsY = (state.value.fieldHeight - 1) * GAP_SIZE;

  const cellByWidth = Math.floor((availableWidth - totalGapsX) / state.value.fieldWidth);
  const cellByHeight = Math.floor((availableHeight - totalGapsY) / state.value.fieldHeight);

  const size = Math.min(cellByWidth, cellByHeight);
  return Math.max(MIN_CELL_SIZE, Math.min(MAX_CELL_SIZE, size));
});

const fontSize = computed(() => {
  const size = cellSize.value;
  if (size >= 48) return "text-4xl";
  if (size >= 36) return "text-3xl";
  if (size >= 28) return "text-2xl";
  return "text-xl";
});

interface LineCoords {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

const winningLineCoords = computed((): LineCoords | null => {
  if (!winningLine.value) return null;
  const line = winningLine.value;
  const size = cellSize.value;
  return {
    x1: line.startX * (size + GAP_SIZE) + size / 2,
    y1: line.startY * (size + GAP_SIZE) + size / 2,
    x2: line.endX * (size + GAP_SIZE) + size / 2,
    y2: line.endY * (size + GAP_SIZE) + size / 2,
  };
});

const gridWidth = computed(() => {
  if (!state.value) return 0;
  return state.value.fieldWidth * cellSize.value + (state.value.fieldWidth - 1) * GAP_SIZE;
});

const gridHeight = computed(() => {
  if (!state.value) return 0;
  return state.value.fieldHeight * cellSize.value + (state.value.fieldHeight - 1) * GAP_SIZE;
});

const isMyTurn = computed(() => {
  if (!state.value || !connectionStore.clientId) return false;
  return state.value.currentPlayer?.playerId === connectionStore.clientId;
});

const myMark = computed(() => {
  if (!state.value || !connectionStore.clientId) return null;
  if (state.value.playerX?.playerId === connectionStore.clientId) return MarkType.X;
  if (state.value.playerO?.playerId === connectionStore.clientId) return MarkType.O;
  return null;
});

const gameInProgress = computed(() => {
  return state.value?.status === GameStatus.IN_PROGRESS;
});

const statusMessage = computed(() => {
  if (!state.value) return "";

  switch (state.value.status) {
    case GameStatus.IN_PROGRESS:
      if (isMyTurn.value) {
        return "Your turn";
      }
      return `Waiting for ${state.value.currentPlayer?.playerId ?? "opponent"}`;
    case GameStatus.X_WON:
      return `${state.value.playerX?.playerId ?? "X"} wins!`;
    case GameStatus.O_WON:
      return `${state.value.playerO?.playerId ?? "O"} wins!`;
    case GameStatus.DRAW:
      return "Draw!";
    default:
      return "";
  }
});

interface GridCell {
  x: number;
  y: number;
  key: string;
}

const gridCells = computed((): GridCell[] => {
  if (!state.value) return [];

  const cells: GridCell[] = [];
  for (let y = 0; y < state.value.fieldHeight; y++) {
    for (let x = 0; x < state.value.fieldWidth; x++) {
      cells.push({ x, y, key: `cell-${x}-${y}` });
    }
  }
  return cells;
});

function getCellMark(x: number, y: number): MarkType {
  if (!state.value) return MarkType.EMPTY;
  const cell = state.value.board.find((c) => c.x === x && c.y === y);
  return cell?.mark ?? MarkType.EMPTY;
}

function isLastMove(x: number, y: number): boolean {
  if (!state.value?.lastMove) return false;
  return state.value.lastMove.x === x && state.value.lastMove.y === y;
}

function handleCellClick(x: number, y: number): void {
  if (!gameInProgress.value || !isMyTurn.value) return;
  if (getCellMark(x, y) !== MarkType.EMPTY) return;

  gameStore.sendTicTacToeCommand(x, y);
}

function formatPlayerName(playerId: string, isBot: boolean): string {
  if (isBot) return `${playerId} [BOT]`;
  return playerId;
}

function getCellClasses(x: number, y: number): Record<string, boolean> {
  const mark = getCellMark(x, y);
  const isEmpty = mark === MarkType.EMPTY;
  const canClick = gameInProgress.value && isMyTurn.value && isEmpty;

  return {
    "cursor-pointer hover:bg-gray-600": canClick,
    "cursor-not-allowed": !canClick,
    "bg-gray-600": isLastMove(x, y),
  };
}

let resizeObserver: ResizeObserver | null = null;

function updateContainerSize(): void {
  if (containerRef.value) {
    containerSize.value = {
      width: containerRef.value.clientWidth,
      height: window.innerHeight - 120,
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
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});
</script>

<template>
  <div ref="containerRef" class="flex flex-col lg:flex-row gap-2 lg:gap-4 items-center lg:items-start w-full">
    <div class="flex-shrink-0">
      <div class="bg-gray-800 p-0 lg:p-3 rounded-lg">
        <div v-if="state" class="relative">
          <div
            class="grid"
            :style="{
              gridTemplateColumns: `repeat(${state.fieldWidth}, ${cellSize}px)`,
              gap: `${GAP_SIZE}px`,
            }"
          >
            <button
              v-for="cell in gridCells"
              :key="cell.key"
              class="bg-gray-700 rounded flex items-center justify-center font-bold transition-colors"
              :class="[getCellClasses(cell.x, cell.y), fontSize]"
              :style="{ width: `${cellSize}px`, height: `${cellSize}px` }"
              @click="handleCellClick(cell.x, cell.y)"
            >
              <span
                v-if="getCellMark(cell.x, cell.y) === MarkType.X"
                class="text-blue-400"
              >
                X
              </span>
              <span
                v-else-if="getCellMark(cell.x, cell.y) === MarkType.O"
                class="text-red-400"
              >
                O
              </span>
            </button>
          </div>

          <svg
            v-if="winningLineCoords"
            class="absolute inset-0 pointer-events-none"
            :width="gridWidth"
            :height="gridHeight"
          >
            <line
              :x1="winningLineCoords.x1"
              :y1="winningLineCoords.y1"
              :x2="winningLineCoords.x2"
              :y2="winningLineCoords.y2"
              stroke="#facc15"
              :stroke-width="Math.max(4, cellSize / 10)"
              stroke-linecap="round"
            />
          </svg>
        </div>
      </div>

      <div
        class="mt-2 lg:mt-4 text-center text-base lg:text-lg font-medium"
        :class="{
          'text-green-400': isMyTurn && gameInProgress,
          'text-gray-400': !isMyTurn && gameInProgress,
          'text-yellow-400': !gameInProgress,
        }"
      >
        {{ statusMessage }}
      </div>
    </div>

    <div class="w-full lg:flex-grow lg:min-w-64">
      <div class="bg-gray-800 rounded-lg p-3 lg:p-4">
        <h3 class="text-base lg:text-lg font-semibold mb-3 lg:mb-4 text-gray-200">Players</h3>

        <div v-if="state" class="space-y-2 lg:space-y-3">
          <div
            class="flex items-center justify-between p-2 lg:p-3 rounded-lg"
            :class="{
              'bg-blue-900/30 ring-2 ring-blue-500':
                state.currentPlayer?.playerId === state.playerX?.playerId && gameInProgress,
              'bg-gray-700': !(state.currentPlayer?.playerId === state.playerX?.playerId && gameInProgress),
            }"
          >
            <div class="flex items-center gap-2 lg:gap-3">
              <span class="text-2xl lg:text-3xl font-bold text-blue-400">X</span>
              <span
                class="font-medium text-sm lg:text-base"
                :class="{
                  'text-gray-200': state.playerX?.playerId === connectionStore.clientId,
                  'text-gray-400': state.playerX?.playerId !== connectionStore.clientId,
                }"
              >
                {{ formatPlayerName(state.playerX?.playerId ?? "Unknown", state.playerX?.isBot ?? false) }}
                <span
                  v-if="state.playerX?.playerId === connectionStore.clientId"
                  class="text-xs text-green-400 ml-1"
                >
                  (you)
                </span>
              </span>
            </div>
          </div>

          <div
            class="flex items-center justify-between p-2 lg:p-3 rounded-lg"
            :class="{
              'bg-red-900/30 ring-2 ring-red-500':
                state.currentPlayer?.playerId === state.playerO?.playerId && gameInProgress,
              'bg-gray-700': !(state.currentPlayer?.playerId === state.playerO?.playerId && gameInProgress),
            }"
          >
            <div class="flex items-center gap-2 lg:gap-3">
              <span class="text-2xl lg:text-3xl font-bold text-red-400">O</span>
              <span
                class="font-medium text-sm lg:text-base"
                :class="{
                  'text-gray-200': state.playerO?.playerId === connectionStore.clientId,
                  'text-gray-400': state.playerO?.playerId !== connectionStore.clientId,
                }"
              >
                {{ formatPlayerName(state.playerO?.playerId ?? "Unknown", state.playerO?.isBot ?? false) }}
                <span
                  v-if="state.playerO?.playerId === connectionStore.clientId"
                  class="text-xs text-green-400 ml-1"
                >
                  (you)
                </span>
              </span>
            </div>
          </div>
        </div>

        <div v-if="myMark !== null" class="mt-3 lg:mt-4 pt-3 lg:pt-4 border-t border-gray-700">
          <div class="text-xs lg:text-sm text-gray-400">
            <span>You are playing as </span>
            <span
              class="font-bold text-base lg:text-lg"
              :class="{
                'text-blue-400': myMark === MarkType.X,
                'text-red-400': myMark === MarkType.O,
              }"
            >
              {{ myMark === MarkType.X ? "X" : "O" }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
