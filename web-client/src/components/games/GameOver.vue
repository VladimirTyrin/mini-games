<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useGameStore } from "../../stores/game";
import { useConnectionStore } from "../../stores/connection";
import { useLobbyStore } from "../../stores/lobby";
import { useDeviceStore } from "../../stores/device";

const router = useRouter();
const gameStore = useGameStore();
const connectionStore = useConnectionStore();
const lobbyStore = useLobbyStore();
const deviceStore = useDeviceStore();

const gameOver = computed(() => gameStore.gameOver);
const playAgainStatus = computed(() => gameStore.playAgainStatus);
const canPlayAgain = computed(() => gameStore.canPlayAgain);
const hasVotedPlayAgain = computed(() => gameStore.hasVotedPlayAgain);

const gameType = computed(() => gameStore.gameType);
const winner = computed(() => gameOver.value?.winner);

const isCooperativeGame = computed(() => {
  return gameType.value === "stackAttack" || gameType.value === "numbersMatch";
});

const isWinner = computed(() => {
  if (!winner.value || !connectionStore.clientId) return false;
  return winner.value.playerId === connectionStore.clientId;
});

const isDraw = computed(() => {
  if (isCooperativeGame.value) return false;
  return gameOver.value && !winner.value;
});

const sortedScores = computed(() => {
  if (!gameOver.value) return [];
  return [...gameOver.value.scores].sort((a, b) => b.score - a.score);
});

const resultMessage = computed(() => {
  if (isCooperativeGame.value) return "Game Over";
  if (isDraw.value) return "Draw!";
  if (isWinner.value) return "You Win!";
  if (winner.value) {
    const name = formatPlayerName(winner.value.playerId, winner.value.isBot);
    return `${name} Wins!`;
  }
  return "Game Over";
});

const resultClass = computed(() => {
  if (isCooperativeGame.value) return "text-orange-400";
  if (isDraw.value) return "text-yellow-400";
  if (isWinner.value) return "text-green-400";
  return "text-red-400";
});

function formatPlayerName(playerId: string, isBot: boolean): string {
  if (isBot) return `${playerId} [BOT]`;
  return playerId;
}

function handlePlayAgain(): void {
  gameStore.playAgain();
}

function handleLeave(): void {
  gameStore.leaveGame();
  lobbyStore.leaveLobby();
  router.push("/");
}
</script>

<template>
  <div class="flex flex-col items-center">
    <div class="bg-gray-800 rounded-lg p-8 max-w-md w-full">
      <h2
        class="text-4xl font-bold text-center mb-6"
        :class="resultClass"
      >
        {{ resultMessage }}
      </h2>

      <div v-if="sortedScores.length > 0" class="mb-6">
        <h3 class="text-lg font-semibold mb-3 text-gray-200">Final Scores</h3>

        <div class="space-y-2">
          <div
            v-for="(entry, index) in sortedScores"
            :key="entry.identity?.playerId"
            class="flex items-center justify-between p-3 rounded-lg"
            :class="{
              'bg-yellow-900/30': index === 0 && !isDraw,
              'bg-gray-700': index !== 0 || isDraw,
              'ring-2 ring-green-500': entry.identity?.playerId === connectionStore.clientId,
            }"
          >
            <div class="flex items-center gap-3">
              <span
                v-if="index === 0 && !isDraw"
                class="text-2xl"
              >
                1st
              </span>
              <span
                v-else-if="index === 1"
                class="text-xl text-gray-400"
              >
                2nd
              </span>
              <span
                v-else-if="index === 2"
                class="text-lg text-gray-500"
              >
                3rd
              </span>
              <span
                v-else
                class="text-gray-500"
              >
                {{ index + 1 }}th
              </span>

              <span class="font-medium text-gray-200">
                {{ formatPlayerName(entry.identity?.playerId ?? "Unknown", entry.identity?.isBot ?? false) }}
                <span
                  v-if="entry.identity?.playerId === connectionStore.clientId"
                  class="text-xs text-green-400 ml-1"
                >
                  (you)
                </span>
              </span>
            </div>

            <span class="font-bold text-xl text-gray-200">
              {{ entry.score }}
            </span>
          </div>
        </div>
      </div>

      <div v-if="playAgainStatus && canPlayAgain" class="mb-6">
        <h3 class="text-lg font-semibold mb-3 text-gray-200">Play Again?</h3>

        <div class="space-y-2 text-sm">
          <div
            v-for="player in playAgainStatus.readyPlayers"
            :key="player.playerId"
            class="flex items-center gap-2 text-green-400"
          >
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path
                fill-rule="evenodd"
                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                clip-rule="evenodd"
              />
            </svg>
            <span>{{ formatPlayerName(player.playerId, player.isBot) }}</span>
          </div>

          <div
            v-for="player in playAgainStatus.pendingPlayers"
            :key="player.playerId"
            class="flex items-center gap-2 text-gray-400"
          >
            <svg class="w-4 h-4 animate-pulse" fill="currentColor" viewBox="0 0 20 20">
              <path
                fill-rule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z"
                clip-rule="evenodd"
              />
            </svg>
            <span>Waiting for {{ formatPlayerName(player.playerId, player.isBot) }}...</span>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-3">
        <button
          v-if="canPlayAgain && !hasVotedPlayAgain"
          class="w-full px-4 py-3 bg-green-600 hover:bg-green-700 text-white font-semibold rounded-lg transition-colors"
          @click="handlePlayAgain"
        >
          Play Again<template v-if="!deviceStore.isTouchDevice"> (Enter)</template>
        </button>

        <button
          v-if="canPlayAgain && hasVotedPlayAgain"
          class="w-full px-4 py-3 bg-gray-600 text-gray-300 font-semibold rounded-lg cursor-not-allowed"
          disabled
        >
          Waiting for others...
        </button>

        <div
          v-if="!canPlayAgain"
          class="text-center text-gray-400 text-sm py-2"
        >
          Play again is not available (some players have left)
        </div>

        <button
          class="w-full px-4 py-3 bg-gray-700 hover:bg-gray-600 text-white font-semibold rounded-lg transition-colors"
          @click="handleLeave"
        >
          Leave Game<template v-if="!deviceStore.isTouchDevice"> (Escape)</template>
        </button>
      </div>
    </div>
  </div>
</template>
