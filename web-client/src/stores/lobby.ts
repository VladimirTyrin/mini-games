import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { create } from "@bufbuild/protobuf";
import type {
  LobbyInfo,
  LobbyDetails,
  LobbyListResponse,
  LobbyCreatedNotification,
  LobbyJoinedNotification,
  LobbyUpdateNotification,
  PlayerJoinedNotification,
  PlayerLeftNotification,
  PlayerReadyNotification,
  KickedFromLobbyNotification,
  LobbyClosedNotification,
  LobbySettings,
  PlayerBecameObserverNotification,
  ObserverBecamePlayerNotification,
} from "../proto/game_service_pb";
import { LobbySettingsSchema } from "../proto/game_service_pb";
import { SnakeBotType, SnakeLobbySettingsSchema } from "../proto/games/snake_pb";
import { TicTacToeBotType, TicTacToeLobbySettingsSchema } from "../proto/games/tictactoe_pb";
import { NumbersMatchLobbySettingsSchema } from "../proto/games/numbers_match_pb";
import { StackAttackLobbySettingsSchema } from "../proto/games/stack_attack_pb";
import { gameClient } from "../api/client";
import { useConnectionStore } from "./connection";

export const useLobbyStore = defineStore("lobby", () => {
  const lobbies = ref<LobbyInfo[]>([]);
  const currentLobby = ref<LobbyDetails | null>(null);
  const kickReason = ref<string | null>(null);
  const closedMessage = ref<string | null>(null);

  const connectionStore = useConnectionStore();

  const isHost = computed(() => {
    if (!currentLobby.value || !connectionStore.clientId) return false;
    return currentLobby.value.creator?.playerId === connectionStore.clientId;
  });

  const isReady = computed(() => {
    if (!currentLobby.value || !connectionStore.clientId) return false;
    const player = currentLobby.value.players.find(
      (p) => p.identity?.playerId === connectionStore.clientId
    );
    return player?.ready ?? false;
  });

  const isObserver = computed(() => {
    if (!currentLobby.value || !connectionStore.clientId) return false;
    return currentLobby.value.observers.some(
      (o) => o.playerId === connectionStore.clientId
    );
  });

  const canStart = computed(() => {
    if (!currentLobby.value || !isHost.value) return false;
    const players = currentLobby.value.players;
    const allReady = players.every((p) => p.ready);
    if (!allReady) return false;

    const settingsCase = currentLobby.value.settings.case;
    if (settingsCase === "tictactoe") {
      return players.length === 2;
    } else if (settingsCase === "numbersMatch") {
      return players.length === 1;
    } else if (settingsCase === "stackAttack") {
      return players.length >= 1 && players.length <= 4;
    } else {
      return players.length >= 1;
    }
  });

  const gameType = computed(() => {
    if (!currentLobby.value) return null;
    return currentLobby.value.settings.case ?? null;
  });

  function refreshLobbies(): void {
    gameClient.listLobbies();
  }

  function createLobby(
    lobbyName: string,
    maxPlayers: number,
    settings: LobbySettings
  ): void {
    gameClient.createLobby(lobbyName, maxPlayers, settings);
  }

  function createSnakeLobby(
    lobbyName: string,
    maxPlayers: number,
    snakeSettings: Parameters<typeof create<typeof SnakeLobbySettingsSchema>>[1]
  ): void {
    const settings = create(LobbySettingsSchema, {
      settings: {
        case: "snake",
        value: create(SnakeLobbySettingsSchema, snakeSettings),
      },
    });
    gameClient.createLobby(lobbyName, maxPlayers, settings);
  }

  function createTicTacToeLobby(
    lobbyName: string,
    maxPlayers: number,
    tttSettings: Parameters<typeof create<typeof TicTacToeLobbySettingsSchema>>[1]
  ): void {
    const settings = create(LobbySettingsSchema, {
      settings: {
        case: "tictactoe",
        value: create(TicTacToeLobbySettingsSchema, tttSettings),
      },
    });
    gameClient.createLobby(lobbyName, maxPlayers, settings);
  }

  function createNumbersMatchLobby(
    lobbyName: string,
    nmSettings: Parameters<typeof create<typeof NumbersMatchLobbySettingsSchema>>[1]
  ): void {
    const settings = create(LobbySettingsSchema, {
      settings: {
        case: "numbersMatch",
        value: create(NumbersMatchLobbySettingsSchema, nmSettings),
      },
    });
    gameClient.createLobby(lobbyName, 1, settings);
  }

  function createStackAttackLobby(lobbyName: string): void {
    const settings = create(LobbySettingsSchema, {
      settings: {
        case: "stackAttack",
        value: create(StackAttackLobbySettingsSchema, {}),
      },
    });
    gameClient.createLobby(lobbyName, 4, settings);
  }

  function joinLobby(lobbyId: string, asObserver = false): void {
    kickReason.value = null;
    closedMessage.value = null;
    gameClient.joinLobby(lobbyId, asObserver);
  }

  function leaveLobby(): void {
    gameClient.leaveLobby();
    currentLobby.value = null;
  }

  function markReady(ready: boolean): void {
    gameClient.markReady(ready);
  }

  function startGame(): void {
    gameClient.startGame();
  }

  function addSnakeBot(botType: SnakeBotType): void {
    gameClient.addBot({ case: "snakeBot", value: botType });
  }

  function addTicTacToeBot(botType: TicTacToeBotType): void {
    gameClient.addBot({ case: "tictactoeBot", value: botType });
  }

  function kickPlayer(playerId: string): void {
    gameClient.kickFromLobby(playerId);
  }

  function becomeObserver(): void {
    gameClient.becomeObserver();
  }

  function becomePlayer(): void {
    gameClient.becomePlayer();
  }

  function makePlayerObserver(playerId: string): void {
    gameClient.makePlayerObserver(playerId);
  }

  function handleLobbyList(response: LobbyListResponse): void {
    lobbies.value = response.lobbies;
  }

  function handleLobbyCreated(notification: LobbyCreatedNotification): void {
    if (notification.details) {
      currentLobby.value = notification.details;
    }
  }

  function handleLobbyJoined(notification: LobbyJoinedNotification): void {
    if (notification.details) {
      currentLobby.value = notification.details;
    }
  }

  function handleLobbyUpdate(notification: LobbyUpdateNotification): void {
    if (notification.details) {
      currentLobby.value = notification.details;
    }
  }

  function handlePlayerJoined(notification: PlayerJoinedNotification): void {
    if (!currentLobby.value || !notification.player) return;

    const existingPlayer = currentLobby.value.players.find(
      (p) => p.identity?.playerId === notification.player?.playerId
    );
    if (!existingPlayer) {
      currentLobby.value.players.push({
        identity: notification.player,
        ready: false,
      } as (typeof currentLobby.value.players)[0]);
    }
  }

  function handlePlayerLeft(notification: PlayerLeftNotification): void {
    if (!currentLobby.value || !notification.player) return;

    currentLobby.value.players = currentLobby.value.players.filter(
      (p) => p.identity?.playerId !== notification.player?.playerId
    );

    currentLobby.value.observers = currentLobby.value.observers.filter(
      (o) => o.playerId !== notification.player?.playerId
    );
  }

  function handlePlayerReady(notification: PlayerReadyNotification): void {
    if (!currentLobby.value || !notification.player) return;

    const player = currentLobby.value.players.find(
      (p) => p.identity?.playerId === notification.player?.playerId
    );
    if (player) {
      player.ready = notification.ready;
    }
  }

  function handleKicked(notification: KickedFromLobbyNotification): void {
    kickReason.value = notification.reason;
    currentLobby.value = null;
  }

  function handleLobbyClosed(notification: LobbyClosedNotification): void {
    closedMessage.value = notification.message;
    currentLobby.value = null;
  }

  function handlePlayerBecameObserver(
    notification: PlayerBecameObserverNotification
  ): void {
    if (!currentLobby.value || !notification.player) return;

    currentLobby.value.players = currentLobby.value.players.filter(
      (p) => p.identity?.playerId !== notification.player?.playerId
    );

    if (
      !currentLobby.value.observers.some(
        (o) => o.playerId === notification.player?.playerId
      )
    ) {
      currentLobby.value.observers.push(notification.player);
    }
  }

  function handleObserverBecamePlayer(
    notification: ObserverBecamePlayerNotification
  ): void {
    if (!currentLobby.value || !notification.observer) return;

    currentLobby.value.observers = currentLobby.value.observers.filter(
      (o) => o.playerId !== notification.observer?.playerId
    );

    if (
      !currentLobby.value.players.some(
        (p) => p.identity?.playerId === notification.observer?.playerId
      )
    ) {
      currentLobby.value.players.push({
        identity: notification.observer,
        ready: false,
      } as (typeof currentLobby.value.players)[0]);
    }
  }

  function clearKickReason(): void {
    kickReason.value = null;
  }

  function clearClosedMessage(): void {
    closedMessage.value = null;
  }

  return {
    lobbies,
    currentLobby,
    kickReason,
    closedMessage,
    isHost,
    isReady,
    isObserver,
    canStart,
    gameType,
    refreshLobbies,
    createLobby,
    createSnakeLobby,
    createTicTacToeLobby,
    createNumbersMatchLobby,
    createStackAttackLobby,
    joinLobby,
    leaveLobby,
    markReady,
    startGame,
    addSnakeBot,
    addTicTacToeBot,
    kickPlayer,
    becomeObserver,
    becomePlayer,
    makePlayerObserver,
    handleLobbyList,
    handleLobbyCreated,
    handleLobbyJoined,
    handleLobbyUpdate,
    handlePlayerJoined,
    handlePlayerLeft,
    handlePlayerReady,
    handleKicked,
    handleLobbyClosed,
    handlePlayerBecameObserver,
    handleObserverBecamePlayer,
    clearKickReason,
    clearClosedMessage,
  };
});
