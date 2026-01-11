<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter } from "vue-router";
import { useLobbyStore } from "../stores/lobby";
import { useConnectionStore } from "../stores/connection";
import { useGameStore } from "../stores/game";
import { useChatStore, type ChatMessage } from "../stores/chat";
import { useToastStore } from "../stores/toast";
import { useDeviceStore } from "../stores/device";
import { SnakeBotType, WallCollisionMode, DeadSnakeBehavior } from "../proto/games/snake_pb";
import { TicTacToeBotType, FirstPlayerMode } from "../proto/games/tictactoe_pb";

const router = useRouter();
const lobbyStore = useLobbyStore();
const connectionStore = useConnectionStore();
const gameStore = useGameStore();
const chatStore = useChatStore();
const toastStore = useToastStore();
const deviceStore = useDeviceStore();

const chatInput = ref("");
const chatContainer = ref<HTMLElement | null>(null);

const lobby = computed(() => lobbyStore.currentLobby);
const isHost = computed(() => lobbyStore.isHost);
const isReady = computed(() => lobbyStore.isReady);
const isObserver = computed(() => lobbyStore.isObserver);
const canStart = computed(() => lobbyStore.canStart);
const gameType = computed(() => lobbyStore.gameType);
const clientId = computed(() => connectionStore.clientId);
const chatMessages = computed(() => chatStore.inLobbyMessages);

const gameTypeBadge = computed(() => {
  if (gameType.value === "snake") return { text: "Snake", class: "bg-green-600" };
  if (gameType.value === "tictactoe") return { text: "TicTacToe", class: "bg-blue-600" };
  return { text: "Unknown", class: "bg-gray-600" };
});


const canAddBot = computed(() => {
  if (!lobby.value || !isHost.value) return false;
  return lobby.value.players.length < lobby.value.maxPlayers;
});

const canBecomePlayer = computed(() => {
  if (!lobby.value || !isObserver.value) return false;
  return lobby.value.players.length < lobby.value.maxPlayers;
});

function isCurrentPlayer(playerId: string | undefined): boolean {
  return playerId === clientId.value;
}

function isHostPlayer(playerId: string | undefined): boolean {
  return playerId === lobby.value?.creator?.playerId;
}

function toggleReady(): void {
  lobbyStore.markReady(!isReady.value);
}

function leaveLobby(): void {
  lobbyStore.leaveLobby();
  chatStore.clearInLobbyMessages();
  router.push("/");
}

function startGame(): void {
  if (canStart.value) {
    lobbyStore.startGame();
  }
}

function addBot(): void {
  if (gameType.value === "snake") {
    lobbyStore.addSnakeBot(SnakeBotType.EFFICIENT);
  } else if (gameType.value === "tictactoe") {
    lobbyStore.addTicTacToeBot(TicTacToeBotType.TICTACTOE_BOT_TYPE_MINIMAX);
  }
}

function kickPlayer(playerId: string): void {
  lobbyStore.kickPlayer(playerId);
}

function makeObserver(playerId: string): void {
  lobbyStore.makePlayerObserver(playerId);
}

function handleBecomeObserver(): void {
  lobbyStore.becomeObserver();
}

function handleBecomePlayer(): void {
  lobbyStore.becomePlayer();
}

function sendChatMessage(): void {
  if (chatInput.value.trim()) {
    chatStore.sendInLobbyMessage(chatInput.value.trim());
    chatInput.value = "";
  }
}

function formatPlayerName(message: ChatMessage): string {
  const name = message.sender.playerId;
  return message.sender.isBot ? `[BOT] ${name}` : name;
}

function scrollChatToBottom(): void {
  nextTick(() => {
    if (chatContainer.value) {
      chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
    }
  });
}

function getWallCollisionModeLabel(mode: WallCollisionMode): string {
  switch (mode) {
    case WallCollisionMode.DEATH: return "Death";
    case WallCollisionMode.WRAP_AROUND: return "Wrap Around";
    default: return "Unknown";
  }
}

function getDeadSnakeBehaviorLabel(behavior: DeadSnakeBehavior): string {
  switch (behavior) {
    case DeadSnakeBehavior.DISAPPEAR: return "Disappear";
    case DeadSnakeBehavior.STAY_ON_FIELD: return "Stay on Field";
    default: return "Unknown";
  }
}

function getFirstPlayerModeLabel(mode: FirstPlayerMode): string {
  switch (mode) {
    case FirstPlayerMode.RANDOM: return "Random";
    case FirstPlayerMode.HOST: return "Host";
    default: return "Unknown";
  }
}

function handleKeydown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement;
  const isInputFocused = target.tagName === "INPUT" || target.tagName === "TEXTAREA";

  if (event.key === "Escape") {
    leaveLobby();
    return;
  }

  if (isInputFocused) return;

  const isCtrl = event.ctrlKey || event.metaKey;

  if (isCtrl && (event.key === "r" || event.key === "R")) {
    event.preventDefault();
    if (!isObserver.value) {
      toggleReady();
    }
    return;
  }

  if (isCtrl && (event.key === "o" || event.key === "O")) {
    event.preventDefault();
    if (!isObserver.value) {
      handleBecomeObserver();
    }
    return;
  }

  if (isCtrl && (event.key === "p" || event.key === "P")) {
    event.preventDefault();
    if (isObserver.value && canBecomePlayer.value) {
      handleBecomePlayer();
    }
    return;
  }

  if (isCtrl && (event.key === "s" || event.key === "S")) {
    event.preventDefault();
    if (isHost.value && canStart.value) {
      startGame();
    }
    return;
  }

  if (isCtrl && (event.key === "b" || event.key === "B")) {
    event.preventDefault();
    if (isHost.value && canAddBot.value) {
      addBot();
    }
    return;
  }
}

watch(() => gameStore.isInGame, (inGame) => {
  if (inGame) {
    router.push("/game");
  }
});

watch(() => lobbyStore.kickReason, (reason) => {
  if (reason) {
    toastStore.error(`You were kicked: ${reason}`);
    lobbyStore.clearKickReason();
    router.push("/");
  }
});

watch(() => lobbyStore.closedMessage, (message) => {
  if (message) {
    toastStore.error(`Lobby closed: ${message}`);
    lobbyStore.clearClosedMessage();
    router.push("/");
  }
});

watch(() => lobby.value, (currentLobby) => {
  if (!currentLobby) {
    router.push("/");
  }
});

watch(chatMessages, () => {
  scrollChatToBottom();
}, { deep: true });

onMounted(() => {
  if (!lobby.value) {
    router.push("/");
  }
  document.addEventListener("keydown", handleKeydown);
  scrollChatToBottom();
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div v-if="lobby" class="bg-gray-900 text-white p-4">
    <div class="max-w-6xl mx-auto">
      <!-- Header -->
      <div class="flex items-center justify-between mb-6">
        <div class="flex items-center gap-4">
          <h1 class="text-3xl font-bold">{{ lobby.lobbyName }}</h1>
          <span
            :class="[gameTypeBadge.class, 'px-3 py-1 rounded-full text-sm font-medium']"
          >
            {{ gameTypeBadge.text }}
          </span>
        </div>
        <button
          class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg transition-colors"
          @click="leaveLobby"
        >
          Leave Lobby<template v-if="!deviceStore.isTouchDevice"> (Esc)</template>
        </button>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Left Column: Players & Observers -->
        <div class="lg:col-span-2 space-y-6">
          <!-- Players List -->
          <div class="bg-gray-800 rounded-lg p-4">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-xl font-semibold">
                Players ({{ lobby.players.length }}/{{ lobby.maxPlayers }})
              </h2>

              <!-- Host Controls: Add Bot -->
              <button
                v-if="isHost && canAddBot"
                class="px-3 py-1 bg-purple-600 hover:bg-purple-700 rounded transition-colors text-sm"
                @click="addBot"
              >
                Add Bot<template v-if="!deviceStore.isTouchDevice"> (Ctrl+B)</template>
              </button>
            </div>

            <div class="space-y-2">
              <div
                v-for="player in lobby.players"
                :key="player.identity?.playerId"
                :class="[
                  'flex items-center justify-between p-3 rounded-lg',
                  isCurrentPlayer(player.identity?.playerId)
                    ? 'bg-blue-900/50 border border-blue-500'
                    : 'bg-gray-700',
                ]"
              >
                <div class="flex items-center gap-3">
                  <!-- Ready Status -->
                  <span
                    :class="[
                      'w-6 h-6 flex items-center justify-center rounded-full text-sm',
                      player.ready ? 'bg-green-600' : 'bg-red-600',
                    ]"
                  >
                    {{ player.ready ? "&#10003;" : "&#10005;" }}
                  </span>

                  <!-- Player Name -->
                  <span class="font-medium">
                    <span v-if="player.identity?.isBot" class="text-purple-400">[BOT] </span>
                    {{ player.identity?.playerId }}
                    <span v-if="isCurrentPlayer(player.identity?.playerId)" class="text-blue-400"> (You)</span>
                  </span>

                  <!-- Host Badge -->
                  <span
                    v-if="isHostPlayer(player.identity?.playerId)"
                    class="px-2 py-0.5 bg-yellow-600 rounded text-xs"
                    title="Host"
                  >
                    Host
                  </span>
                </div>

                <!-- Host Controls (not for self) -->
                <div v-if="isHost && !isCurrentPlayer(player.identity?.playerId)" class="flex gap-1">
                  <button
                    v-if="!player.identity?.isBot"
                    class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-sm transition-colors"
                    @click="makeObserver(player.identity?.playerId ?? '')"
                    title="Move to observers"
                  >
                    Observe
                  </button>
                  <button
                    class="px-2 py-1 bg-red-600 hover:bg-red-700 rounded text-sm transition-colors"
                    @click="kickPlayer(player.identity?.playerId ?? '')"
                  >
                    Kick
                  </button>
                </div>
              </div>

              <div
                v-if="lobby.players.length === 0"
                class="text-gray-400 text-center py-4"
              >
                No players yet
              </div>
            </div>
          </div>

          <!-- Observers Section -->
          <div class="bg-gray-800 rounded-lg p-4">
            <h2 class="text-xl font-semibold mb-4">
              Observers ({{ lobby.observers.length }})
            </h2>

            <div class="space-y-2">
              <div
                v-for="observer in lobby.observers"
                :key="observer.playerId"
                :class="[
                  'flex items-center justify-between p-3 rounded-lg',
                  isCurrentPlayer(observer.playerId)
                    ? 'bg-blue-900/50 border border-blue-500'
                    : 'bg-gray-700',
                ]"
              >
                <span class="font-medium">
                  {{ observer.playerId }}
                  <span v-if="isCurrentPlayer(observer.playerId)" class="text-blue-400"> (You)</span>
                </span>
              </div>

              <div
                v-if="lobby.observers.length === 0"
                class="text-gray-400 text-center py-4"
              >
                No observers
              </div>
            </div>

            <!-- Observer/Player Toggle Buttons -->
            <div class="mt-4 flex gap-2">
              <button
                v-if="!isObserver"
                class="px-3 py-2 bg-gray-600 hover:bg-gray-500 rounded transition-colors"
                @click="handleBecomeObserver"
              >
                Become Observer<template v-if="!deviceStore.isTouchDevice"> (Ctrl+O)</template>
              </button>
              <button
                v-if="isObserver && canBecomePlayer"
                class="px-3 py-2 bg-green-600 hover:bg-green-700 rounded transition-colors"
                @click="handleBecomePlayer"
              >
                Become Player<template v-if="!deviceStore.isTouchDevice"> (Ctrl+P)</template>
              </button>
              <span
                v-if="isObserver && !canBecomePlayer"
                class="text-gray-400 text-sm self-center"
              >
                Lobby is full
              </span>
            </div>
          </div>

          <!-- Player Controls -->
          <div v-if="!isObserver || isHost" class="bg-gray-800 rounded-lg p-4">
            <h2 class="text-xl font-semibold mb-4">Controls</h2>
            <div class="flex gap-4">
              <button
                v-if="!isObserver"
                :class="[
                  'px-6 py-3 rounded-lg font-medium transition-colors',
                  isReady
                    ? 'bg-red-600 hover:bg-red-700'
                    : 'bg-green-600 hover:bg-green-700',
                ]"
                @click="toggleReady"
              >
                {{ isReady ? "Not Ready" : "Ready" }}<template v-if="!deviceStore.isTouchDevice"> (Ctrl+R)</template>
              </button>

              <button
                v-if="isHost"
                :disabled="!canStart"
                :class="[
                  'px-6 py-3 rounded-lg font-medium transition-colors',
                  canStart
                    ? 'bg-blue-600 hover:bg-blue-700'
                    : 'bg-gray-600 cursor-not-allowed',
                ]"
                @click="startGame"
              >
                Start Game<template v-if="!deviceStore.isTouchDevice"> (Ctrl+S)</template>
              </button>
            </div>
            <p v-if="isHost && !canStart" class="text-gray-400 mt-2 text-sm">
              <template v-if="!lobby.players.every(p => p.ready)">
                All players must be ready.
              </template>
              <template v-else-if="gameType === 'tictactoe' && lobby.players.length !== 2">
                TicTacToe requires exactly 2 players.
              </template>
              <template v-else-if="lobby.players.length < 1">
                At least 1 player required.
              </template>
            </p>
          </div>
        </div>

        <!-- Right Column: Settings & Chat -->
        <div class="space-y-6">
          <!-- Settings Display -->
          <div class="bg-gray-800 rounded-lg p-4">
            <h2 class="text-xl font-semibold mb-4">Game Settings</h2>

            <!-- Snake Settings -->
            <div v-if="lobby.settings.case === 'snake'" class="space-y-2 text-sm">
              <div class="flex justify-between">
                <span class="text-gray-400">Field Size:</span>
                <span>{{ lobby.settings.value.fieldWidth }} x {{ lobby.settings.value.fieldHeight }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Tick Rate:</span>
                <span>{{ lobby.settings.value.tickIntervalMs }}ms</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Wall Collision:</span>
                <span>{{ getWallCollisionModeLabel(lobby.settings.value.wallCollisionMode) }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Max Food:</span>
                <span>{{ lobby.settings.value.maxFoodCount }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Food Spawn Rate:</span>
                <span>{{ (lobby.settings.value.foodSpawnProbability * 100).toFixed(0) }}%</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Dead Snake:</span>
                <span>{{ getDeadSnakeBehaviorLabel(lobby.settings.value.deadSnakeBehavior) }}</span>
              </div>
            </div>

            <!-- TicTacToe Settings -->
            <div v-else-if="lobby.settings.case === 'tictactoe'" class="space-y-2 text-sm">
              <div class="flex justify-between">
                <span class="text-gray-400">Board Size:</span>
                <span>{{ lobby.settings.value.fieldWidth }} x {{ lobby.settings.value.fieldHeight }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">Win Count:</span>
                <span>{{ lobby.settings.value.winCount }} in a row</span>
              </div>
              <div class="flex justify-between">
                <span class="text-gray-400">First Player:</span>
                <span>{{ getFirstPlayerModeLabel(lobby.settings.value.firstPlayer) }}</span>
              </div>
            </div>
          </div>

          <!-- Chat Section -->
          <div class="bg-gray-800 rounded-lg p-4 flex flex-col h-80">
            <h2 class="text-xl font-semibold mb-4">Lobby Chat</h2>

            <div
              ref="chatContainer"
              class="flex-1 overflow-y-auto space-y-2 mb-4"
            >
              <div
                v-for="(msg, index) in chatMessages"
                :key="index"
                class="text-sm"
              >
                <span class="font-medium text-blue-400">{{ formatPlayerName(msg) }}:</span>
                <span class="text-gray-300"> {{ msg.message }}</span>
              </div>
              <div
                v-if="chatMessages.length === 0"
                class="text-gray-400 text-center py-4"
              >
                No messages yet
              </div>
            </div>

            <div class="flex gap-2">
              <input
                v-model="chatInput"
                type="text"
                placeholder="Type a message..."
                class="flex-1 px-3 py-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 focus:outline-none"
                @keyup.enter="sendChatMessage"
              />
              <button
                class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded transition-colors"
                @click="sendChatMessage"
              >
                Send
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Loading/Redirect State -->
  <div v-else class="bg-gray-900 text-white flex items-center justify-center">
    <p class="text-gray-400">Loading lobby...</p>
  </div>
</template>
