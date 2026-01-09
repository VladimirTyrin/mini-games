import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { gameClient, type ConnectionState } from "../api/client";
import { useConfigStore } from "./config";
import { useLobbyStore } from "./lobby";
import { useGameStore } from "./game";
import { useChatStore } from "./chat";

const CLIENT_ID_STORAGE_KEY = "mini_games_client_id";

export const useConnectionStore = defineStore("connection", () => {
  const state = ref<ConnectionState>("disconnected");
  const clientId = ref<string | null>(loadClientId());
  const error = ref<string | null>(null);

  function loadClientId(): string | null {
    return localStorage.getItem(CLIENT_ID_STORAGE_KEY);
  }

  function saveClientId(id: string): void {
    localStorage.setItem(CLIENT_ID_STORAGE_KEY, id);
  }

  function clearClientId(): void {
    localStorage.removeItem(CLIENT_ID_STORAGE_KEY);
    clientId.value = null;
  }

  const isConnected = computed(() => state.value === "connected");

  gameClient.onConnectionStateChange = (newState) => {
    state.value = newState;
  };

  gameClient.onError = (message) => {
    error.value = message;
  };

  gameClient.onServerMessage = (msg) => {
    const lobbyStore = useLobbyStore();
    const gameStore = useGameStore();
    const chatStore = useChatStore();

    console.log("[WS] Received message:", msg.message.case);

    switch (msg.message.case) {
      case "connect":
        if (!msg.message.value.success) {
          error.value = "Connection rejected by server";
          disconnect();
        }
        break;

      case "lobbyList":
        console.log("[WS] Received lobbyList with", msg.message.value.lobbies.length, "lobbies");
        lobbyStore.handleLobbyList(msg.message.value);
        break;

      case "lobbyCreated":
        lobbyStore.handleLobbyCreated(msg.message.value);
        break;

      case "lobbyJoined":
        lobbyStore.handleLobbyJoined(msg.message.value);
        break;

      case "lobbyUpdate":
        lobbyStore.handleLobbyUpdate(msg.message.value);
        break;

      case "playerJoined":
        lobbyStore.handlePlayerJoined(msg.message.value);
        break;

      case "playerLeft":
        lobbyStore.handlePlayerLeft(msg.message.value);
        break;

      case "playerReady":
        lobbyStore.handlePlayerReady(msg.message.value);
        break;

      case "kicked":
        lobbyStore.handleKicked(msg.message.value);
        break;

      case "lobbyClosed":
        lobbyStore.handleLobbyClosed(msg.message.value);
        break;

      case "lobbyListUpdate":
        console.log("[WS] Received lobbyListUpdate notification, refreshing...");
        lobbyStore.refreshLobbies();
        break;

      case "playerBecameObserver":
        lobbyStore.handlePlayerBecameObserver(msg.message.value);
        break;

      case "observerBecamePlayer":
        lobbyStore.handleObserverBecamePlayer(msg.message.value);
        break;

      case "gameStarting":
        gameStore.handleGameStarting(msg.message.value);
        break;

      case "gameState":
        gameStore.handleGameState(msg.message.value);
        break;

      case "gameOver":
        gameStore.handleGameOver(msg.message.value);
        break;

      case "playAgainStatus":
        gameStore.handlePlayAgainStatus(msg.message.value);
        break;

      case "lobbyListChat":
        chatStore.handleLobbyListChat(msg.message.value);
        break;

      case "inLobbyChat":
        chatStore.handleInLobbyChat(msg.message.value);
        break;

      case "shutdown":
        error.value = msg.message.value.message || "Server is shutting down";
        disconnect();
        break;
    }
  };

  async function connect(id: string): Promise<void> {
    if (state.value === "connected" || state.value === "connecting") {
      return;
    }

    clientId.value = id;
    saveClientId(id);
    error.value = null;

    const configStore = useConfigStore();
    await gameClient.connect(configStore.serverUrl, id);
  }

  function disconnect(): void {
    gameClient.disconnect();
    error.value = null;
  }

  return {
    state,
    clientId,
    error,
    isConnected,
    connect,
    disconnect,
    clearClientId,
  };
});

export type { ConnectionState };
