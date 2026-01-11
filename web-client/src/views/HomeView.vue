<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useConnectionStore } from "../stores/connection";
import { useLobbyStore } from "../stores/lobby";
import { useConfigStore } from "../stores/config";
import { useDeviceStore } from "../stores/device";
import {
  WallCollisionMode,
  DeadSnakeBehavior,
} from "../proto/games/snake_pb";
import { FirstPlayerMode } from "../proto/games/tictactoe_pb";

type GameType = "snake" | "tictactoe";

const router = useRouter();
const connectionStore = useConnectionStore();
const lobbyStore = useLobbyStore();
const configStore = useConfigStore();
const deviceStore = useDeviceStore();

const username = ref("");
const isConnecting = ref(false);
const showCreateDialog = ref(false);

const newLobbyName = ref("");
const newLobbyGameType = ref<GameType>("snake");
const newLobbyMaxPlayers = ref(4);

const snakeFieldWidth = ref(configStore.snakeDefaults.fieldWidth);
const snakeFieldHeight = ref(configStore.snakeDefaults.fieldHeight);
const snakeTickInterval = ref(configStore.snakeDefaults.tickIntervalMs);
const snakeWallCollision = ref(configStore.snakeDefaults.wallCollisionMode);
const snakeMaxFood = ref(configStore.snakeDefaults.maxFoodCount);
const snakeFoodSpawnProb = ref(configStore.snakeDefaults.foodSpawnProbability);
const snakeDeadSnakeBehavior = ref(configStore.snakeDefaults.deadSnakeBehavior);

const tttFieldWidth = ref(configStore.tictactoeDefaults.fieldWidth);
const tttFieldHeight = ref(configStore.tictactoeDefaults.fieldHeight);
const tttWinCount = ref(configStore.tictactoeDefaults.winCount);
const tttFirstPlayer = ref(configStore.tictactoeDefaults.firstPlayer);

const savedClientId = connectionStore.clientId;
if (savedClientId) {
  username.value = savedClientId;
}

const isConnected = computed(() => connectionStore.isConnected);
const connectionState = computed(() => connectionStore.state);
const connectionError = computed(() => connectionStore.error);
const lobbies = computed(() => lobbyStore.lobbies);

function getGameTypeLabel(lobby: (typeof lobbies.value)[0]): string {
  const settings = lobby.settings?.settings;
  if (!settings) return "Unknown";
  if (settings.case === "snake") return "Snake";
  if (settings.case === "tictactoe") return "TicTacToe";
  return "Unknown";
}

function getGameTypeIcon(lobby: (typeof lobbies.value)[0]): string {
  const settings = lobby.settings?.settings;
  if (!settings) return "?";
  if (settings.case === "snake") return "S";
  if (settings.case === "tictactoe") return "X";
  return "?";
}

async function handleConnect() {
  if (!username.value.trim()) return;

  isConnecting.value = true;
  try {
    await connectionStore.connect(username.value.trim());
  } finally {
    isConnecting.value = false;
  }
}

function handleRefresh() {
  lobbyStore.refreshLobbies();
}

function handleJoinLobby(lobbyId: string) {
  lobbyStore.joinLobby(lobbyId, false);
}

function handleJoinAsObserver(lobbyId: string) {
  lobbyStore.joinLobby(lobbyId, true);
}

function openCreateDialog() {
  newLobbyName.value = connectionStore.clientId ?? "";
  newLobbyGameType.value = "snake";
  newLobbyMaxPlayers.value = 4;

  snakeFieldWidth.value = configStore.snakeDefaults.fieldWidth;
  snakeFieldHeight.value = configStore.snakeDefaults.fieldHeight;
  snakeTickInterval.value = configStore.snakeDefaults.tickIntervalMs;
  snakeWallCollision.value = configStore.snakeDefaults.wallCollisionMode;
  snakeMaxFood.value = configStore.snakeDefaults.maxFoodCount;
  snakeFoodSpawnProb.value = configStore.snakeDefaults.foodSpawnProbability;
  snakeDeadSnakeBehavior.value = configStore.snakeDefaults.deadSnakeBehavior;

  tttFieldWidth.value = configStore.tictactoeDefaults.fieldWidth;
  tttFieldHeight.value = configStore.tictactoeDefaults.fieldHeight;
  tttWinCount.value = configStore.tictactoeDefaults.winCount;
  tttFirstPlayer.value = configStore.tictactoeDefaults.firstPlayer;

  showCreateDialog.value = true;
}

function closeCreateDialog() {
  showCreateDialog.value = false;
}

function handleCreateLobby() {
  if (!newLobbyName.value.trim()) return;

  if (newLobbyGameType.value === "snake") {
    lobbyStore.createSnakeLobby(newLobbyName.value.trim(), newLobbyMaxPlayers.value, {
      fieldWidth: snakeFieldWidth.value,
      fieldHeight: snakeFieldHeight.value,
      tickIntervalMs: snakeTickInterval.value,
      wallCollisionMode: snakeWallCollision.value,
      maxFoodCount: snakeMaxFood.value,
      foodSpawnProbability: snakeFoodSpawnProb.value,
      deadSnakeBehavior: snakeDeadSnakeBehavior.value,
    });
  } else {
    lobbyStore.createTicTacToeLobby(newLobbyName.value.trim(), 2, {
      fieldWidth: tttFieldWidth.value,
      fieldHeight: tttFieldHeight.value,
      winCount: tttWinCount.value,
      firstPlayer: tttFirstPlayer.value,
    });
  }

  showCreateDialog.value = false;
}

watch(
  () => lobbyStore.currentLobby,
  (lobby) => {
    if (lobby) {
      router.push({ name: "lobby", params: { id: lobby.lobbyId } });
    }
  }
);

function handleKeyDown(event: KeyboardEvent): void {
  const isCtrl = event.ctrlKey || event.metaKey;

  if (showCreateDialog.value) {
    if (event.key === "Enter") {
      event.preventDefault();
      handleCreateLobby();
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeCreateDialog();
    }
    return;
  }

  if (isConnected.value && isCtrl && (event.key === "n" || event.key === "N")) {
    event.preventDefault();
    openCreateDialog();
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
  <div class="bg-slate-900 text-white p-6">
    <div class="max-w-4xl mx-auto">
      <div class="flex justify-between items-center mb-8">
        <h1 class="text-4xl font-bold">Mini Games</h1>
        <router-link
          to="/settings"
          class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded transition-colors"
        >
          Settings
        </router-link>
      </div>

      <template v-if="!isConnected">
        <div class="bg-slate-800 rounded-lg p-6 max-w-md mx-auto">
          <h2 class="text-xl font-semibold mb-4">Connect to Server</h2>

          <div v-if="connectionError" class="bg-red-900/50 border border-red-500 rounded p-3 mb-4">
            <p class="text-red-300">{{ connectionError }}</p>
          </div>

          <div class="mb-4">
            <label for="username" class="block text-sm font-medium text-slate-300 mb-2">
              Username
            </label>
            <input
              id="username"
              v-model="username"
              type="text"
              class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="Enter your username"
              @keyup.enter="handleConnect"
              :disabled="isConnecting || connectionState === 'connecting'"
            />
          </div>

          <button
            @click="handleConnect"
            :disabled="!username.trim() || isConnecting || connectionState === 'connecting'"
            class="w-full bg-blue-600 hover:bg-blue-500 disabled:bg-slate-600 disabled:cursor-not-allowed text-white font-medium py-2 px-4 rounded transition-colors"
          >
            <span v-if="connectionState === 'connecting'">Connecting...</span>
            <span v-else>Connect</span>
          </button>
        </div>
      </template>

      <template v-else>
        <div class="flex justify-between items-center mb-6">
          <h2 class="text-2xl font-semibold">Available Lobbies</h2>
          <div class="flex gap-3">
            <button
              @click="handleRefresh"
              class="bg-slate-700 hover:bg-slate-600 text-white font-medium py-2 px-4 rounded transition-colors"
            >
              Refresh
            </button>
            <button
              @click="openCreateDialog"
              class="bg-blue-600 hover:bg-blue-500 text-white font-medium py-2 px-4 rounded transition-colors"
            >
              Create Lobby<template v-if="!deviceStore.isTouchDevice"> (Ctrl+N)</template>
            </button>
          </div>
        </div>

        <div v-if="lobbies.length === 0" class="bg-slate-800 rounded-lg p-8 text-center">
          <p class="text-slate-400">No lobbies available. Create one to get started!</p>
        </div>

        <div v-else class="space-y-3">
          <div
            v-for="lobby in lobbies"
            :key="lobby.lobbyId"
            class="bg-slate-800 rounded-lg p-4 flex items-center justify-between"
          >
            <div class="flex items-center gap-4">
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center text-lg font-bold"
                :class="{
                  'bg-green-600': getGameTypeIcon(lobby) === 'S',
                  'bg-purple-600': getGameTypeIcon(lobby) === 'X',
                  'bg-slate-600': getGameTypeIcon(lobby) === '?',
                }"
              >
                {{ getGameTypeIcon(lobby) }}
              </div>
              <div>
                <h3 class="font-semibold text-lg">{{ lobby.lobbyName }}</h3>
                <p class="text-sm text-slate-400">
                  {{ getGameTypeLabel(lobby) }} - {{ lobby.currentPlayers }}/{{ lobby.maxPlayers }} players
                  <span v-if="lobby.observerCount > 0" class="ml-2">
                    ({{ lobby.observerCount }} observers)
                  </span>
                </p>
              </div>
            </div>
            <div class="flex gap-2">
              <button
                @click="handleJoinAsObserver(lobby.lobbyId)"
                class="bg-slate-700 hover:bg-slate-600 text-white font-medium py-2 px-3 rounded text-sm transition-colors"
              >
                Observe
              </button>
              <button
                @click="handleJoinLobby(lobby.lobbyId)"
                :disabled="lobby.currentPlayers >= lobby.maxPlayers"
                class="bg-blue-600 hover:bg-blue-500 disabled:bg-slate-600 disabled:cursor-not-allowed text-white font-medium py-2 px-4 rounded transition-colors"
              >
                Join
              </button>
            </div>
          </div>
        </div>
      </template>
    </div>

    <div
      v-if="showCreateDialog"
      class="fixed inset-0 bg-black/50 flex items-center justify-center p-4 z-50"
      @click.self="closeCreateDialog"
    >
      <div
        class="bg-slate-800 rounded-lg p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto"
        @click.stop
      >
        <h2 class="text-xl font-semibold mb-4">Create Lobby</h2>

        <div class="space-y-4">
          <div>
            <label for="lobbyName" class="block text-sm font-medium text-slate-300 mb-2">
              Lobby Name
            </label>
            <input
              id="lobbyName"
              v-model="newLobbyName"
              type="text"
              class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="Enter lobby name"
            />
          </div>

          <div>
            <label class="block text-sm font-medium text-slate-300 mb-2">Game Type</label>
            <div class="flex gap-3">
              <button
                @click="newLobbyGameType = 'snake'"
                :class="[
                  'flex-1 py-2 px-4 rounded font-medium transition-colors',
                  newLobbyGameType === 'snake'
                    ? 'bg-green-600 text-white'
                    : 'bg-slate-700 text-slate-300 hover:bg-slate-600',
                ]"
              >
                Snake
              </button>
              <button
                @click="newLobbyGameType = 'tictactoe'"
                :class="[
                  'flex-1 py-2 px-4 rounded font-medium transition-colors',
                  newLobbyGameType === 'tictactoe'
                    ? 'bg-purple-600 text-white'
                    : 'bg-slate-700 text-slate-300 hover:bg-slate-600',
                ]"
              >
                TicTacToe
              </button>
            </div>
          </div>

          <div v-if="newLobbyGameType === 'snake'">
            <label for="maxPlayers" class="block text-sm font-medium text-slate-300 mb-2">
              Max Players
            </label>
            <input
              id="maxPlayers"
              v-model.number="newLobbyMaxPlayers"
              type="number"
              min="1"
              max="10"
              class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>

          <template v-if="newLobbyGameType === 'snake'">
            <div class="border-t border-slate-700 pt-4">
              <h3 class="text-lg font-medium mb-3">Snake Settings</h3>

              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label for="snakeWidth" class="block text-sm font-medium text-slate-300 mb-2">
                    Field Width
                  </label>
                  <input
                    id="snakeWidth"
                    v-model.number="snakeFieldWidth"
                    type="number"
                    min="10"
                    max="50"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="snakeHeight" class="block text-sm font-medium text-slate-300 mb-2">
                    Field Height
                  </label>
                  <input
                    id="snakeHeight"
                    v-model.number="snakeFieldHeight"
                    type="number"
                    min="10"
                    max="50"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="tickInterval" class="block text-sm font-medium text-slate-300 mb-2">
                    Tick Interval (ms)
                  </label>
                  <input
                    id="tickInterval"
                    v-model.number="snakeTickInterval"
                    type="number"
                    min="50"
                    max="500"
                    step="10"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="maxFood" class="block text-sm font-medium text-slate-300 mb-2">
                    Max Food
                  </label>
                  <input
                    id="maxFood"
                    v-model.number="snakeMaxFood"
                    type="number"
                    min="1"
                    max="20"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="foodSpawnProb" class="block text-sm font-medium text-slate-300 mb-2">
                    Food Spawn Probability
                  </label>
                  <input
                    id="foodSpawnProb"
                    v-model.number="snakeFoodSpawnProb"
                    type="number"
                    min="0"
                    max="1"
                    step="0.1"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="wallCollision" class="block text-sm font-medium text-slate-300 mb-2">
                    Wall Collision
                  </label>
                  <select
                    id="wallCollision"
                    v-model.number="snakeWallCollision"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  >
                    <option :value="WallCollisionMode.DEATH">Death</option>
                    <option :value="WallCollisionMode.WRAP_AROUND">Wrap Around</option>
                  </select>
                </div>

                <div class="col-span-2">
                  <label for="deadSnakeBehavior" class="block text-sm font-medium text-slate-300 mb-2">
                    Dead Snake Behavior
                  </label>
                  <select
                    id="deadSnakeBehavior"
                    v-model.number="snakeDeadSnakeBehavior"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  >
                    <option :value="DeadSnakeBehavior.DISAPPEAR">Disappear</option>
                    <option :value="DeadSnakeBehavior.STAY_ON_FIELD">Stay on Field</option>
                  </select>
                </div>
              </div>
            </div>
          </template>

          <template v-if="newLobbyGameType === 'tictactoe'">
            <div class="border-t border-slate-700 pt-4">
              <h3 class="text-lg font-medium mb-3">TicTacToe Settings</h3>

              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label for="tttWidth" class="block text-sm font-medium text-slate-300 mb-2">
                    Field Width
                  </label>
                  <input
                    id="tttWidth"
                    v-model.number="tttFieldWidth"
                    type="number"
                    min="3"
                    max="20"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="tttHeight" class="block text-sm font-medium text-slate-300 mb-2">
                    Field Height
                  </label>
                  <input
                    id="tttHeight"
                    v-model.number="tttFieldHeight"
                    type="number"
                    min="3"
                    max="20"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="winCount" class="block text-sm font-medium text-slate-300 mb-2">
                    Win Count
                  </label>
                  <input
                    id="winCount"
                    v-model.number="tttWinCount"
                    type="number"
                    min="3"
                    max="15"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                </div>

                <div>
                  <label for="firstPlayer" class="block text-sm font-medium text-slate-300 mb-2">
                    First Player
                  </label>
                  <select
                    id="firstPlayer"
                    v-model.number="tttFirstPlayer"
                    class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  >
                    <option :value="FirstPlayerMode.RANDOM">Random</option>
                    <option :value="FirstPlayerMode.HOST">Host</option>
                  </select>
                </div>
              </div>
            </div>
          </template>
        </div>

        <div class="flex gap-3 mt-6">
          <button
            @click="closeCreateDialog"
            class="flex-1 bg-slate-700 hover:bg-slate-600 text-white font-medium py-2 px-4 rounded transition-colors"
          >
            Cancel
          </button>
          <button
            @click="handleCreateLobby"
            :disabled="!newLobbyName.trim()"
            class="flex-1 bg-blue-600 hover:bg-blue-500 disabled:bg-slate-600 disabled:cursor-not-allowed text-white font-medium py-2 px-4 rounded transition-colors"
          >
            Create
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
