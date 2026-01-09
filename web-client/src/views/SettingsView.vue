<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useConfigStore } from "../stores/config";
import { useConnectionStore } from "../stores/connection";
import {
  WallCollisionMode,
  DeadSnakeBehavior,
} from "../proto/games/snake_pb";
import { FirstPlayerMode } from "../proto/games/tictactoe_pb";

const router = useRouter();
const configStore = useConfigStore();
const connectionStore = useConnectionStore();

const serverUrl = ref(configStore.serverUrl);

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

const saveMessage = ref("");

function loadFromStore(): void {
  serverUrl.value = configStore.serverUrl;

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
}

function saveSettings(): void {
  configStore.setServerUrl(serverUrl.value);

  configStore.setSnakeDefaults({
    fieldWidth: snakeFieldWidth.value,
    fieldHeight: snakeFieldHeight.value,
    tickIntervalMs: snakeTickInterval.value,
    wallCollisionMode: snakeWallCollision.value,
    maxFoodCount: snakeMaxFood.value,
    foodSpawnProbability: snakeFoodSpawnProb.value,
    deadSnakeBehavior: snakeDeadSnakeBehavior.value,
  });

  configStore.setTicTacToeDefaults({
    fieldWidth: tttFieldWidth.value,
    fieldHeight: tttFieldHeight.value,
    winCount: tttWinCount.value,
    firstPlayer: tttFirstPlayer.value,
  });

  saveMessage.value = "Settings saved!";
  setTimeout(() => {
    saveMessage.value = "";
  }, 2000);
}

function resetToDefaults(): void {
  configStore.reset();
  loadFromStore();
  saveMessage.value = "Settings reset to defaults!";
  setTimeout(() => {
    saveMessage.value = "";
  }, 2000);
}

function goBack(): void {
  router.push("/");
}

onMounted(() => {
  loadFromStore();
});
</script>

<template>
  <div class="bg-slate-900 text-white p-6">
    <div class="max-w-2xl mx-auto">
      <div class="flex items-center justify-between mb-8">
        <h1 class="text-3xl font-bold">Settings</h1>
        <button
          @click="goBack"
          class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded transition-colors"
        >
          Back
        </button>
      </div>

      <div
        v-if="saveMessage"
        class="bg-green-900/50 border border-green-500 rounded p-3 mb-6"
      >
        <p class="text-green-300">{{ saveMessage }}</p>
      </div>

      <div class="space-y-8">
        <div class="bg-slate-800 rounded-lg p-6">
          <h2 class="text-xl font-semibold mb-4">Connection</h2>

          <div>
            <label for="serverUrl" class="block text-sm font-medium text-slate-300 mb-2">
              Server URL
            </label>
            <input
              id="serverUrl"
              v-model="serverUrl"
              type="text"
              class="w-full bg-slate-700 border border-slate-600 rounded px-3 py-2 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="ws://localhost:5000/ws"
            />
            <p class="text-xs text-slate-400 mt-1">
              Note: Changing this requires reconnecting (refresh page).
            </p>
          </div>
        </div>

        <div class="bg-slate-800 rounded-lg p-6">
          <h2 class="text-xl font-semibold mb-4">Snake Defaults</h2>

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

        <div class="bg-slate-800 rounded-lg p-6">
          <h2 class="text-xl font-semibold mb-4">TicTacToe Defaults</h2>

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

        <div class="flex gap-4">
          <button
            @click="saveSettings"
            class="flex-1 bg-blue-600 hover:bg-blue-500 text-white font-medium py-3 px-4 rounded transition-colors"
          >
            Save Settings
          </button>
          <button
            @click="resetToDefaults"
            class="flex-1 bg-slate-700 hover:bg-slate-600 text-white font-medium py-3 px-4 rounded transition-colors"
          >
            Reset to Defaults
          </button>
        </div>

        <div v-if="connectionStore.isConnected" class="text-center">
          <p class="text-slate-400 text-sm">
            Currently connected as <span class="text-blue-400">{{ connectionStore.clientId }}</span>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
