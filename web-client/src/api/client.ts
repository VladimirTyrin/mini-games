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
  DisconnectRequestSchema,
  ErrorCode,
  type InGameCommand,
  InGameCommandSchema,
  InLobbyChatMessageSchema,
  JoinLobbyRequestSchema,
  KickFromLobbyRequestSchema,
  LeaveLobbyRequestSchema,
  ListLobbiesRequestSchema,
  LobbyListChatMessageSchema,
  type LobbySettings,
  MarkReadyRequestSchema,
  PingRequestSchema,
  PlayAgainRequestSchema,
  type ServerMessage,
  StartGameRequestSchema,
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
    await this.ws.connect(url);
    this.sendConnectRequest(clientId);
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

  sendInGameCommand(command: InGameCommand["command"]): void {
    this.sendMessage({
      case: "inGame",
      value: create(InGameCommandSchema, { command }),
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
