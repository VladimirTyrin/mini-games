import { create } from "@bufbuild/protobuf";
import {
  type AddBotRequest,
  AddBotRequestSchema,
  BecomeObserverFromPlayerRequestSchema,
  BecomePlayerFromObserverRequestSchema,
  type ClientMessage,
  ClientMessageSchema,
  ConnectRequestSchema,
  CreateLobbyRequestSchema,
  CreateReplayLobbyRequestSchema,
  DisconnectRequestSchema,
  ErrorCode,
  type InGameCommand,
  InGameCommandSchema,
  type InReplayCommand,
  InReplayCommandSchema,
  InLobbyChatMessageSchema,
  JoinLobbyRequestSchema,
  KickFromLobbyRequestSchema,
  LeaveLobbyRequestSchema,
  ListLobbiesRequestSchema,
  LobbyListChatMessageSchema,
  type LobbySettings,
  MakePlayerObserverRequestSchema,
  MarkReadyRequestSchema,
  PingRequestSchema,
  PlayAgainRequestSchema,
  type ServerMessage,
  StartGameRequestSchema,
  WatchReplayTogetherRequestSchema,
} from "../proto/game_service_pb";
import { type ConnectionState, WebSocketClient } from "./websocket";

export type ServerMessageHandler = (message: ServerMessage) => void;
export type ConnectionStateHandler = (state: ConnectionState) => void;
export type ErrorHandler = (message: string) => void;

export class GameClient {
  private ws: WebSocketClient;
  private pingCounter = 0n;

  onServerMessage: ServerMessageHandler | null = null;
  onConnectionStateChange: ConnectionStateHandler | null = null;
  onError: ErrorHandler | null = null;

  constructor() {
    this.ws = new WebSocketClient();

    this.ws.onMessage = (message) => {
      this.handleServerMessage(message);
    };

    this.ws.onConnectionChange = (state) => {
      this.onConnectionStateChange?.(state);
    };
  }

  get connectionState(): ConnectionState {
    return this.ws.state;
  }

  async connect(url: string, clientId: string): Promise<void> {
    console.log("[Client] Connecting to", url);
    await this.ws.connect(url);
    console.log("[Client] WebSocket open, sending ConnectRequest for", clientId);

    await new Promise<void>((resolve, reject) => {
      const originalHandler = this.ws.onMessage;
      this.ws.onMessage = (message) => {
        if (message.message.case === "connect") {
          const response = message.message.value;
          this.ws.onMessage = originalHandler;
          if (response.success) {
            console.log("[Client] Connected, now listing lobbies");
            resolve();
          } else {
            const errorMessage =
              response.errorMessage || "Connection rejected by server";
            reject(new Error(errorMessage));
          }
        } else if (message.message.case === "error") {
          this.ws.onMessage = originalHandler;
          reject(new Error(message.message.value.message));
        }
        originalHandler?.(message);
      };
      this.sendConnectRequest(clientId);
    });

    this.listLobbies();
  }

  disconnect(): void {
    this.sendMessage({ case: "disconnect", value: create(DisconnectRequestSchema, {}) });
    this.ws.disconnect();
  }

  listLobbies(): void {
    this.sendMessage({ case: "listLobbies", value: create(ListLobbiesRequestSchema, {}) });
  }

  createLobby(name: string, maxPlayers: number, settings: LobbySettings): void {
    this.sendMessage({
      case: "createLobby",
      value: create(CreateLobbyRequestSchema, {
        lobbyName: name,
        maxPlayers,
        settings,
      }),
    });
  }

  joinLobby(lobbyId: string, asObserver = false): void {
    this.sendMessage({
      case: "joinLobby",
      value: create(JoinLobbyRequestSchema, {
        lobbyId,
        joinAsObserver: asObserver,
      }),
    });
  }

  leaveLobby(): void {
    this.sendMessage({ case: "leaveLobby", value: create(LeaveLobbyRequestSchema, {}) });
  }

  markReady(ready: boolean): void {
    this.sendMessage({
      case: "markReady",
      value: create(MarkReadyRequestSchema, { ready }),
    });
  }

  startGame(): void {
    this.sendMessage({ case: "startGame", value: create(StartGameRequestSchema, {}) });
  }

  playAgain(): void {
    this.sendMessage({ case: "playAgain", value: create(PlayAgainRequestSchema, {}) });
  }

  addBot(botType: AddBotRequest["botType"]): void {
    this.sendMessage({
      case: "addBot",
      value: create(AddBotRequestSchema, { botType }),
    });
  }

  kickFromLobby(playerId: string): void {
    this.sendMessage({
      case: "kickFromLobby",
      value: create(KickFromLobbyRequestSchema, { playerId }),
    });
  }

  becomeObserver(): void {
    this.sendMessage({
      case: "becomeObserver",
      value: create(BecomeObserverFromPlayerRequestSchema, {}),
    });
  }

  becomePlayer(): void {
    this.sendMessage({
      case: "becomePlayer",
      value: create(BecomePlayerFromObserverRequestSchema, {}),
    });
  }

  makePlayerObserver(playerId: string): void {
    this.sendMessage({
      case: "makeObserver",
      value: create(MakePlayerObserverRequestSchema, { playerId }),
    });
  }

  sendInGameCommand(command: InGameCommand["command"]): void {
    this.sendMessage({
      case: "inGame",
      value: create(InGameCommandSchema, { command }),
    });
  }

  sendReplayCommand(command: InReplayCommand["command"]): void {
    this.sendMessage({
      case: "inReplay",
      value: create(InReplayCommandSchema, { command }),
    });
  }

  createReplayLobby(replayContent: Uint8Array, hostOnlyControl: boolean): void {
    this.sendMessage({
      case: "createReplayLobby",
      value: create(CreateReplayLobbyRequestSchema, { replayContent, hostOnlyControl }),
    });
  }

  watchReplayTogether(replayContent: Uint8Array, hostOnlyControl: boolean): void {
    this.sendMessage({
      case: "watchReplayTogether",
      value: create(WatchReplayTogetherRequestSchema, { replayContent, hostOnlyControl }),
    });
  }

  sendChat(message: string, inLobby: boolean): void {
    if (inLobby) {
      this.sendMessage({
        case: "inLobbyChat",
        value: create(InLobbyChatMessageSchema, { message }),
      });
    } else {
      this.sendMessage({
        case: "lobbyListChat",
        value: create(LobbyListChatMessageSchema, { message }),
      });
    }
  }

  ping(): void {
    const pingId = this.pingCounter++;
    this.sendMessage({
      case: "ping",
      value: create(PingRequestSchema, {
        pingId,
        clientTimestampMs: BigInt(Date.now()),
      }),
    });
  }

  private sendConnectRequest(clientId: string): void {
    this.sendMessage({
      case: "connect",
      value: create(ConnectRequestSchema, { clientId }),
    });
  }

  private sendMessage(message: ClientMessage["message"]): void {
    const clientMessage = create(ClientMessageSchema, {
      version: __APP_VERSION__,
      message,
    });
    this.ws.send(clientMessage);
  }

  private handleServerMessage(message: ServerMessage): void {
    if (message.message.case === "error") {
      const error = message.message.value;
      console.error("[WS] Server error:", error.code, error.message);
      if (error.code === ErrorCode.VERSION_MISMATCH) {
        this.onError?.("Version mismatch. Please refresh the page.");
        return;
      }
      this.onError?.(error.message);
    }

    this.onServerMessage?.(message);
  }
}

export const gameClient = new GameClient();

export type { ConnectionState, ServerMessage };
export { ErrorCode };
