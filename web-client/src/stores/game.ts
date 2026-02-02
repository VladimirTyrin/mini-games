import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { create } from "@bufbuild/protobuf";
import type {
  GameStartingNotification,
  GameStateUpdate,
  GameOverNotification,
  PlayAgainStatusNotification,
} from "../proto/game_service_pb";
import { InGameCommandSchema } from "../proto/game_service_pb";
import type { SnakeGameState } from "../proto/games/snake_pb";
import { Direction, SnakeInGameCommandSchema, TurnCommandSchema } from "../proto/games/snake_pb";
import type { TicTacToeGameState } from "../proto/games/tictactoe_pb";
import { TicTacToeInGameCommandSchema, PlaceMarkCommandSchema } from "../proto/games/tictactoe_pb";
import type { NumbersMatchGameState } from "../proto/games/numbers_match_pb";
import {
  NumbersMatchInGameCommandSchema,
  RemovePairCommandSchema,
  RefillCommandSchema,
  RequestHintCommandSchema,
} from "../proto/games/numbers_match_pb";
import type { StackAttackGameState } from "../proto/games/stack_attack_pb";
import {
  HorizontalDirection,
  StackAttackInGameCommandSchema,
  MoveCommandSchema,
  JumpCommandSchema,
} from "../proto/games/stack_attack_pb";
import { gameClient } from "../api/client";
import { useConnectionStore } from "./connection";

export type GameType = "snake" | "tictactoe" | "numbersMatch" | "stackAttack" | null;

export const useGameStore = defineStore("game", () => {
  const gameType = ref<GameType>(null);
  const sessionId = ref<string | null>(null);
  const snakeState = ref<SnakeGameState | null>(null);
  const tictactoeState = ref<TicTacToeGameState | null>(null);
  const numbersMatchState = ref<NumbersMatchGameState | null>(null);
  const stackAttackState = ref<StackAttackGameState | null>(null);
  const gameOver = ref<GameOverNotification | null>(null);
  const playAgainStatus = ref<PlayAgainStatusNotification | null>(null);

  const connectionStore = useConnectionStore();

  const isInGame = computed(() => gameType.value !== null && gameOver.value === null);

  const isGameOver = computed(() => gameOver.value !== null);

  const currentState = computed(() => {
    if (gameType.value === "snake") return snakeState.value;
    if (gameType.value === "tictactoe") return tictactoeState.value;
    if (gameType.value === "numbersMatch") return numbersMatchState.value;
    if (gameType.value === "stackAttack") return stackAttackState.value;
    return null;
  });

  const canPlayAgain = computed(() => {
    return playAgainStatus.value?.available ?? false;
  });

  const hasVotedPlayAgain = computed(() => {
    if (!playAgainStatus.value || !connectionStore.clientId) return false;
    return playAgainStatus.value.readyPlayers.some(
      (p) => p.playerId === connectionStore.clientId
    );
  });

  function sendSnakeCommand(direction: Direction): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "snake",
        value: create(SnakeInGameCommandSchema, {
          command: {
            case: "turn",
            value: create(TurnCommandSchema, { direction }),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendTicTacToeCommand(x: number, y: number): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "tictactoe",
        value: create(TicTacToeInGameCommandSchema, {
          command: {
            case: "place",
            value: create(PlaceMarkCommandSchema, { x, y }),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendNumbersMatchRemovePair(firstIndex: number, secondIndex: number): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "numbersMatch",
        value: create(NumbersMatchInGameCommandSchema, {
          command: {
            case: "removePair",
            value: create(RemovePairCommandSchema, { firstIndex, secondIndex }),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendNumbersMatchRefill(): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "numbersMatch",
        value: create(NumbersMatchInGameCommandSchema, {
          command: {
            case: "refill",
            value: create(RefillCommandSchema, {}),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendNumbersMatchRequestHint(): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "numbersMatch",
        value: create(NumbersMatchInGameCommandSchema, {
          command: {
            case: "requestHint",
            value: create(RequestHintCommandSchema, {}),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendStackAttackMove(direction: HorizontalDirection): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "stackAttack",
        value: create(StackAttackInGameCommandSchema, {
          command: {
            case: "move",
            value: create(MoveCommandSchema, { direction }),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function sendStackAttackJump(): void {
    const command = create(InGameCommandSchema, {
      command: {
        case: "stackAttack",
        value: create(StackAttackInGameCommandSchema, {
          command: {
            case: "jump",
            value: create(JumpCommandSchema, {}),
          },
        }),
      },
    });
    gameClient.sendInGameCommand(command.command);
  }

  function playAgain(): void {
    gameClient.playAgain();
  }

  function handleGameStarting(notification: GameStartingNotification): void {
    sessionId.value = notification.sessionId;
    gameOver.value = null;
    playAgainStatus.value = null;
  }

  function handleGameState(update: GameStateUpdate): void {
    switch (update.state.case) {
      case "snake":
        gameType.value = "snake";
        snakeState.value = update.state.value;
        tictactoeState.value = null;
        numbersMatchState.value = null;
        stackAttackState.value = null;
        break;
      case "tictactoe":
        gameType.value = "tictactoe";
        tictactoeState.value = update.state.value;
        snakeState.value = null;
        numbersMatchState.value = null;
        stackAttackState.value = null;
        break;
      case "numbersMatch":
        gameType.value = "numbersMatch";
        numbersMatchState.value = update.state.value;
        snakeState.value = null;
        tictactoeState.value = null;
        stackAttackState.value = null;
        break;
      case "stackAttack":
        gameType.value = "stackAttack";
        stackAttackState.value = update.state.value;
        snakeState.value = null;
        tictactoeState.value = null;
        numbersMatchState.value = null;
        break;
    }
  }

  function handleGameOver(notification: GameOverNotification): void {
    gameOver.value = notification;
  }

  function handlePlayAgainStatus(notification: PlayAgainStatusNotification): void {
    playAgainStatus.value = notification;
    if (notification.available && notification.pendingPlayers.length === 0) {
      resetForNewGame();
    }
  }

  function resetForNewGame(): void {
    gameOver.value = null;
    playAgainStatus.value = null;
  }

  function leaveGame(): void {
    gameType.value = null;
    sessionId.value = null;
    snakeState.value = null;
    tictactoeState.value = null;
    numbersMatchState.value = null;
    stackAttackState.value = null;
    gameOver.value = null;
    playAgainStatus.value = null;
  }

  return {
    gameType,
    sessionId,
    snakeState,
    tictactoeState,
    numbersMatchState,
    stackAttackState,
    gameOver,
    playAgainStatus,
    isInGame,
    isGameOver,
    currentState,
    canPlayAgain,
    hasVotedPlayAgain,
    sendSnakeCommand,
    sendTicTacToeCommand,
    sendNumbersMatchRemovePair,
    sendNumbersMatchRefill,
    sendNumbersMatchRequestHint,
    sendStackAttackMove,
    sendStackAttackJump,
    playAgain,
    handleGameStarting,
    handleGameState,
    handleGameOver,
    handlePlayAgainStatus,
    leaveGame,
  };
});

export { Direction, HorizontalDirection };
