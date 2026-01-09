<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useGameStore } from "../stores/game";
import { useConnectionStore } from "../stores/connection";
import { useLobbyStore } from "../stores/lobby";
import SnakeGame from "../components/games/SnakeGame.vue";
import TicTacToeGame from "../components/games/TicTacToeGame.vue";
import GameOver from "../components/games/GameOver.vue";

const router = useRouter();
const gameStore = useGameStore();
const connectionStore = useConnectionStore();
const lobbyStore = useLobbyStore();

const gameType = computed(() => gameStore.gameType);
const isGameOver = computed(() => gameStore.isGameOver);
const isInGame = computed(() => gameStore.isInGame);
const canPlayAgain = computed(() => gameStore.canPlayAgain);
const hasVotedPlayAgain = computed(() => gameStore.hasVotedPlayAgain);

const gameTitle = computed(() => {
  switch (gameType.value) {
    case "snake":
      return "Snake";
    case "tictactoe":
      return "Tic Tac Toe";
    default:
      return "Game";
  }
});

function handleLeaveGame(): void {
  gameStore.leaveGame();
  lobbyStore.leaveLobby();
  router.push("/");
}

function handleKeyDown(event: KeyboardEvent): void {
  if (isGameOver.value) {
    if (event.key === "Enter" && canPlayAgain.value && !hasVotedPlayAgain.value) {
      event.preventDefault();
      gameStore.playAgain();
    } else if (event.key === "Escape") {
      event.preventDefault();
      handleLeaveGame();
    }
  }
}

onMounted(() => {
  if (!gameType.value) {
    router.push("/");
    return;
  }

  window.addEventListener("keydown", handleKeyDown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
});
</script>

<template>
  <div class="bg-gray-900 text-white">
    <header class="bg-gray-800 border-b border-gray-700">
      <div class="container mx-auto px-4 py-3 flex items-center justify-between">
        <div class="flex items-center gap-4">
          <h1 class="text-xl font-bold text-gray-200">{{ gameTitle }}</h1>
          <span
            v-if="isGameOver"
            class="px-2 py-1 bg-red-600 text-white text-xs font-semibold rounded"
          >
            GAME OVER
          </span>
          <span
            v-else-if="isInGame"
            class="px-2 py-1 bg-green-600 text-white text-xs font-semibold rounded"
          >
            IN PROGRESS
          </span>
        </div>

        <div class="flex items-center gap-4">
          <span class="text-sm text-gray-400">
            Playing as
            <span class="font-medium text-gray-200">{{ connectionStore.clientId }}</span>
          </span>

          <button
            v-if="!isGameOver"
            class="px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded transition-colors"
            @click="handleLeaveGame"
          >
            Leave Game
          </button>
        </div>
      </div>
    </header>

    <main class="container mx-auto px-4 py-6 relative">
      <div v-if="!gameType" class="text-center py-12">
        <p class="text-gray-400 text-lg">No active game. Redirecting...</p>
      </div>

      <div v-else class="relative">
        <div v-if="gameType === 'snake'">
          <SnakeGame />
        </div>

        <div v-else-if="gameType === 'tictactoe'">
          <TicTacToeGame :show-winning-line="isGameOver" />
        </div>

        <div
          v-if="isGameOver"
          class="absolute inset-0 flex items-center justify-center bg-black/60"
        >
          <GameOver />
        </div>
      </div>
    </main>
  </div>
</template>
