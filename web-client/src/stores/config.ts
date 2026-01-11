import { defineStore } from "pinia";
import { ref } from "vue";
import {
  WallCollisionMode,
  DeadSnakeBehavior,
} from "../proto/games/snake_pb";
import { FirstPlayerMode } from "../proto/games/tictactoe_pb";

const CONFIG_STORAGE_KEY = "mini_games_config";

export interface SnakeDefaults {
  fieldWidth: number;
  fieldHeight: number;
  wallCollisionMode: WallCollisionMode;
  tickIntervalMs: number;
  maxFoodCount: number;
  foodSpawnProbability: number;
  deadSnakeBehavior: DeadSnakeBehavior;
}

export interface TicTacToeDefaults {
  fieldWidth: number;
  fieldHeight: number;
  winCount: number;
  firstPlayer: FirstPlayerMode;
}

export interface StoredConfig {
  serverUrl: string;
  snakeDefaults: SnakeDefaults;
  tictactoeDefaults: TicTacToeDefaults;
}

function getDefaultServerUrl(): string {
  if (import.meta.env.VITE_SERVER_URL) {
    return import.meta.env.VITE_SERVER_URL;
  }
  if (typeof window !== "undefined") {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = window.location.hostname;
    if (host === "localhost" || host === "127.0.0.1") {
      return `${protocol}//${host}:5000/ws`;
    }
    return `${protocol}//${host}/ws`;
  }
  return "ws://localhost:5000/ws";
}

function getDefaultSnakeSettings(): SnakeDefaults {
  return {
    fieldWidth: 15,
    fieldHeight: 15,
    wallCollisionMode: WallCollisionMode.WRAP_AROUND,
    tickIntervalMs: 200,
    maxFoodCount: 1,
    foodSpawnProbability: 1.0,
    deadSnakeBehavior: DeadSnakeBehavior.DISAPPEAR,
  };
}

function getDefaultTicTacToeSettings(): TicTacToeDefaults {
  return {
    fieldWidth: 15,
    fieldHeight: 15,
    winCount: 5,
    firstPlayer: FirstPlayerMode.RANDOM,
  };
}

function loadConfig(): StoredConfig | null {
  const stored = localStorage.getItem(CONFIG_STORAGE_KEY);
  if (!stored) return null;

  try {
    return JSON.parse(stored) as StoredConfig;
  } catch {
    return null;
  }
}

function saveConfig(config: StoredConfig): void {
  localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(config));
}

export const useConfigStore = defineStore("config", () => {
  const storedConfig = loadConfig();

  const serverUrl = ref<string>(storedConfig?.serverUrl ?? getDefaultServerUrl());
  const snakeDefaults = ref<SnakeDefaults>(
    storedConfig?.snakeDefaults ?? getDefaultSnakeSettings()
  );
  const tictactoeDefaults = ref<TicTacToeDefaults>(
    storedConfig?.tictactoeDefaults ?? getDefaultTicTacToeSettings()
  );

  function persist(): void {
    saveConfig({
      serverUrl: serverUrl.value,
      snakeDefaults: snakeDefaults.value,
      tictactoeDefaults: tictactoeDefaults.value,
    });
  }

  function setServerUrl(url: string): void {
    serverUrl.value = url;
    persist();
  }

  function setSnakeDefaults(defaults: Partial<SnakeDefaults>): void {
    snakeDefaults.value = { ...snakeDefaults.value, ...defaults };
    persist();
  }

  function setTicTacToeDefaults(defaults: Partial<TicTacToeDefaults>): void {
    tictactoeDefaults.value = { ...tictactoeDefaults.value, ...defaults };
    persist();
  }

  function reset(): void {
    serverUrl.value = getDefaultServerUrl();
    snakeDefaults.value = getDefaultSnakeSettings();
    tictactoeDefaults.value = getDefaultTicTacToeSettings();
    localStorage.removeItem(CONFIG_STORAGE_KEY);
  }

  return {
    serverUrl,
    snakeDefaults,
    tictactoeDefaults,
    setServerUrl,
    setSnakeDefaults,
    setTicTacToeDefaults,
    reset,
  };
});
