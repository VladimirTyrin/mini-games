<script setup lang="ts">
import { computed } from "vue";
import { useGameStore } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";
import { MarkType, GameStatus } from "../../proto/games/tictactoe_pb";

const gameStore = useGameStore();
const connectionStore = useConnectionStore();

const state = computed(() => gameStore.tictactoeState);

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
</script>

<template>
  <div class="flex flex-col lg:flex-row gap-6 items-start">
    <div class="flex-shrink-0">
      <div class="bg-gray-800 p-4 rounded-lg">
        <div
          v-if="state"
          class="grid gap-2"
          :style="{
            gridTemplateColumns: `repeat(${state.fieldWidth}, 80px)`,
          }"
        >
          <button
            v-for="cell in gridCells"
            :key="cell.key"
            class="w-20 h-20 bg-gray-700 rounded-lg flex items-center justify-center text-5xl font-bold transition-colors"
            :class="getCellClasses(cell.x, cell.y)"
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
      </div>

      <div
        class="mt-4 text-center text-lg font-medium"
        :class="{
          'text-green-400': isMyTurn && gameInProgress,
          'text-gray-400': !isMyTurn && gameInProgress,
          'text-yellow-400': !gameInProgress,
        }"
      >
        {{ statusMessage }}
      </div>
    </div>

    <div class="flex-grow min-w-64">
      <div class="bg-gray-800 rounded-lg p-4">
        <h3 class="text-lg font-semibold mb-4 text-gray-200">Players</h3>

        <div v-if="state" class="space-y-3">
          <div
            class="flex items-center justify-between p-3 rounded-lg"
            :class="{
              'bg-blue-900/30 ring-2 ring-blue-500':
                state.currentPlayer?.playerId === state.playerX?.playerId && gameInProgress,
              'bg-gray-700': !(state.currentPlayer?.playerId === state.playerX?.playerId && gameInProgress),
            }"
          >
            <div class="flex items-center gap-3">
              <span class="text-3xl font-bold text-blue-400">X</span>
              <span
                class="font-medium"
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
            class="flex items-center justify-between p-3 rounded-lg"
            :class="{
              'bg-red-900/30 ring-2 ring-red-500':
                state.currentPlayer?.playerId === state.playerO?.playerId && gameInProgress,
              'bg-gray-700': !(state.currentPlayer?.playerId === state.playerO?.playerId && gameInProgress),
            }"
          >
            <div class="flex items-center gap-3">
              <span class="text-3xl font-bold text-red-400">O</span>
              <span
                class="font-medium"
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

        <div v-if="myMark !== null" class="mt-4 pt-4 border-t border-gray-700">
          <div class="text-sm text-gray-400">
            <span>You are playing as </span>
            <span
              class="font-bold text-lg"
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
