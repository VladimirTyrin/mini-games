<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useGameStore } from "../stores/game";
import { useConnectionStore } from "../stores/connection";
import { useLobbyStore } from "../stores/lobby";
import { useChatStore } from "../stores/chat";
import SnakeGame from "../components/games/SnakeGame.vue";
import TicTacToeGame from "../components/games/TicTacToeGame.vue";
import NumbersMatchGame from "../components/games/NumbersMatchGame.vue";
import StackAttackGame from "../components/games/StackAttackGame.vue";
import Puzzle2048Game from "../components/games/Puzzle2048Game.vue";
import GameOver from "../components/games/GameOver.vue";
import ReplayControls from "../components/games/ReplayControls.vue";
import InGameChat from "../components/games/InGameChat.vue";
import { useReplayStore } from "../stores/replay";

const router = useRouter();
const gameStore = useGameStore();
const connectionStore = useConnectionStore();
const lobbyStore = useLobbyStore();
const chatStore = useChatStore();
const replayStore = useReplayStore();

const gameType = computed(() => gameStore.gameType);
const isGameOver = computed(() => gameStore.isGameOver);
const isInGame = computed(() => gameStore.isInGame);
const canPlayAgain = computed(() => gameStore.canPlayAgain);
const hasVotedPlayAgain = computed(() => gameStore.hasVotedPlayAgain);
const isInReplay = computed(() => replayStore.isInReplay);
const humanCountInLobby = computed(() => {
  const lobby = lobbyStore.currentLobby;
  if (!lobby) return 0;
  const players = lobby.players.filter((p) => p.identity && !p.identity.isBot).length;
  const observers = lobby.observers.filter((o) => !o.isBot).length;
  return players + observers;
});
const canStartWatchReplay = computed(() => {
  if (!replayStore.replayData) return false;
  if (humanCountInLobby.value >= 2) return lobbyStore.isHost;
  return true;
});

const showChat = computed(() => {
  const lobby = lobbyStore.currentLobby;
  if (!lobby) return false;
  const humanCount = lobby.players.filter(p => p.identity && !p.identity.isBot).length
    + lobby.observers.filter(o => !o.isBot).length;
  return humanCount >= 2;
});

const gameTitle = computed(() => {
  switch (gameType.value) {
    case "snake":
      return "Snake";
    case "tictactoe":
      return "Tic Tac Toe";
    case "numbersMatch":
      return "Numbers Match";
    case "stackAttack":
      return "Stack Attack";
    case "puzzle2048":
      return "2048";
    default:
      return "Game";
  }
});

function handleLeaveGame(): void {
  gameStore.leaveGame();
  lobbyStore.leaveLobby();
  chatStore.clearInLobbyMessages();
  router.push("/");
}

function handleKeyDown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement | null;
  const isInputFocused =
    target !== null &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable);
  if (isInputFocused) return;

  if (isGameOver.value && !isInReplay.value) {
    if (event.code === "Enter" && canPlayAgain.value && !hasVotedPlayAgain.value) {
      event.preventDefault();
      gameStore.playAgain();
    } else if (event.code === "Escape") {
      event.preventDefault();
      handleLeaveGame();
    } else if (event.code === "KeyW" && replayStore.replayData && canStartWatchReplay.value) {
      event.preventDefault();
      if (humanCountInLobby.value >= 2) {
        replayStore.watchReplayTogether(replayStore.replayData.content, false);
      } else {
        lobbyStore.leaveLobby();
        replayStore.createReplayLobby(replayStore.replayData.content, false);
      }
    } else if (event.code === "KeyS" && replayStore.replayData) {
      event.preventDefault();
      replayStore.saveReplay();
    }
  }
  if (isInReplay.value && event.code === "Escape") {
    event.preventDefault();
    handleLeaveGame();
  }
}

function handlePopState(): void {
  window.history.pushState(null, "", window.location.href);
}

onMounted(() => {
  if (!gameType.value) {
    router.push("/");
    return;
  }

  window.addEventListener("keydown", handleKeyDown);

  window.history.pushState(null, "", window.location.href);
  window.addEventListener("popstate", handlePopState);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
  window.removeEventListener("popstate", handlePopState);
});
</script>

<template>
  <div class="bg-gray-900 text-white">
    <header class="bg-gray-800 border-b border-gray-700">
      <div class="container mx-auto px-4 py-3 flex items-center justify-between">
        <div class="flex items-center gap-4">
          <h1 class="text-xl font-bold text-gray-200">{{ gameTitle }}</h1>
          <span
            v-if="isInReplay"
            class="px-2 py-1 bg-indigo-600 text-white text-xs font-semibold rounded"
          >
            REPLAY
          </span>
          <span
            v-else-if="isGameOver"
            class="px-2 py-1 bg-red-600 text-white text-xs font-semibold rounded"
          >
            GAME OVER
          </span>
          <span
            v-else-if="isInGame"
            class="px-2 py-1 bg-green-600 text-white text-xs font-semibold rounded"
          >
            IN GAME
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

    <main class="container mx-auto px-1 py-1 sm:px-4 sm:py-6 relative">
      <div v-if="!gameType" class="text-center py-12">
        <p class="text-gray-400 text-lg">No active game. Redirecting...</p>
      </div>

      <div v-else class="relative">
        <InGameChat v-if="showChat" />

        <div v-if="gameType === 'snake'">
          <SnakeGame />
        </div>

        <div v-else-if="gameType === 'tictactoe'">
          <TicTacToeGame :show-winning-line="isGameOver" />
        </div>

        <div v-else-if="gameType === 'numbersMatch'">
          <NumbersMatchGame />
        </div>

        <div v-else-if="gameType === 'stackAttack'">
          <StackAttackGame />
        </div>

        <div v-else-if="gameType === 'puzzle2048'">
          <Puzzle2048Game />
        </div>

        <div
          v-if="isGameOver && !isInReplay"
          class="absolute inset-0 flex items-center justify-center"
        >
          <GameOver />
        </div>
      </div>

      <ReplayControls v-if="isInReplay" />
    </main>
  </div>
</template>
