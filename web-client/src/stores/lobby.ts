import { defineStore } from "pinia";
import { computed, ref, shallowRef, watch } from "vue";

import type { BotKind, GameSettings } from "@/proto/minigames/v2/game_pb";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import {
  type ChatMessage,
  ChatScope,
  LobbyClosedReason,
  LobbyPhase,
  type LobbyState,
  type LobbySummary,
} from "@/proto/minigames/v2/lobby_pb";
import type { ServerPayload } from "@/net/session";

import { useConnectionStore } from "./connection";
import { useToastStore } from "./toasts";

const MAX_CHAT_MESSAGES = 200;

export function sameIdentity(a: Identity | undefined | null, b: Identity | undefined | null): boolean {
  return !!a && !!b && a.kind === b.kind && a.id === b.id;
}

const CLOSED_REASONS: Record<LobbyClosedReason, string> = {
  [LobbyClosedReason.UNSPECIFIED]: "The lobby was closed",
  [LobbyClosedReason.HOST_LEFT]: "The lobby was closed",
  [LobbyClosedReason.KICKED]: "You were removed from the lobby",
  [LobbyClosedReason.INACTIVE]: "The lobby was closed for inactivity",
  [LobbyClosedReason.SERVER_SHUTDOWN]: "The server is restarting",
};

export const useLobbyStore = defineStore("lobby", () => {
  const connection = useConnectionStore();
  const toasts = useToastStore();

  const lobbies = ref<LobbySummary[]>([]);
  const current = shallowRef<LobbyState | null>(null);
  const lobbyChat = ref<ChatMessage[]>([]);
  const lobbyListChat = ref<ChatMessage[]>([]);

  const isHost = computed(() => sameIdentity(current.value?.host, connection.identity));
  const mySeatIndex = computed(() =>
    current.value
      ? current.value.seats.findIndex((seat) => sameIdentity(seat.occupant, connection.identity))
      : -1,
  );

  let detach: (() => void)[] = [];
  watch(
    () => connection.session,
    (session) => {
      for (const off of detach) off();
      detach = [];
      if (!session) {
        current.value = null;
        return;
      }
      detach.push(
        session.events.on("lobbyList", (list) => (lobbies.value = list.lobbies)),
        session.events.on("lobbyState", (state) => accept(state)),
        session.events.on("lobbyClosed", (closed) => {
          current.value = null;
          lobbyChat.value = [];
          toasts.show(CLOSED_REASONS[closed.reason] ?? CLOSED_REASONS[LobbyClosedReason.UNSPECIFIED]);
        }),
        session.events.on("chat", (message) => {
          const target = message.scope === ChatScope.LOBBY ? lobbyChat : lobbyListChat;
          target.value = [...target.value.slice(-(MAX_CHAT_MESSAGES - 1)), message];
        }),
        session.events.on("signedIn", () => void refresh()),
      );
    },
    { immediate: true },
  );

  /** Ignores late updates for a lobby we already left. */
  function accept(state: LobbyState): void {
    const known = current.value;
    if (known && known.lobbyId !== state.lobbyId) return;
    if (!known && !state.seats.some((seat) => sameIdentity(seat.occupant, connection.identity))) {
      if (!state.observers.some((observer) => sameIdentity(observer, connection.identity))) return;
    }
    current.value = state;
  }

  async function call(
    message: Parameters<ReturnType<typeof connection.requireSession>["request"]>[0],
  ): Promise<ServerPayload> {
    try {
      return await connection.requireSession().request(message);
    } catch (error) {
      toasts.failure(error);
      throw error;
    }
  }

  async function refresh(): Promise<void> {
    const reply = await call({ case: "listLobbies", value: {} }).catch(() => null);
    if (reply?.case === "lobbyList") lobbies.value = reply.value.lobbies;
  }

  async function create(name: string, settings: GameSettings, maxSeats: number): Promise<void> {
    const reply = await call({ case: "createLobby", value: { name, settings, maxSeats } });
    if (reply.case === "lobbyState") {
      current.value = reply.value;
      lobbyChat.value = [];
    }
  }

  async function join(lobbyId: string, asObserver: boolean): Promise<void> {
    const reply = await call({ case: "joinLobby", value: { lobbyId, asObserver } });
    if (reply.case === "lobbyState") {
      current.value = reply.value;
      lobbyChat.value = [];
    }
  }

  async function leave(): Promise<void> {
    current.value = null;
    lobbyChat.value = [];
    await call({ case: "leaveLobby", value: {} }).catch(() => undefined);
    await refresh();
  }

  const setReady = (ready: boolean) => call({ case: "setReady", value: { ready } });
  const addBot = (botKind: BotKind) => call({ case: "addBot", value: { botKind } });
  const removeSeat = (seat: number) => call({ case: "removeSeat", value: { seat } });
  const kick = (member: Identity) => call({ case: "kickMember", value: { member } });
  const switchToObserver = () => call({ case: "switchToObserver", value: {} });
  const switchToPlayer = () => call({ case: "switchToPlayer", value: {} });
  const start = () => call({ case: "startMatch", value: {} });
  const votePlayAgain = () => call({ case: "votePlayAgain", value: {} });
  const updateSettings = (settings: GameSettings, maxSeats: number) =>
    call({ case: "updateLobbySettings", value: { settings, maxSeats } });
  const sendChat = (scope: ChatScope, text: string) => call({ case: "sendChat", value: { scope, text } });

  const inMatch = computed(() => current.value?.phase === LobbyPhase.IN_MATCH);

  return {
    lobbies,
    current,
    lobbyChat,
    lobbyListChat,
    isHost,
    mySeatIndex,
    inMatch,
    refresh,
    create,
    join,
    leave,
    setReady,
    addBot,
    removeSeat,
    kick,
    switchToObserver,
    switchToPlayer,
    start,
    votePlayAgain,
    updateSettings,
    sendChat,
  };
});
