import { defineStore } from "pinia";
import { computed, ref, shallowRef, watch } from "vue";

import type { BotKind, GameSettings } from "@/proto/minigames/v2/game_pb";
import {
  type ChatMessage,
  ChatScope,
  LobbyClosedReason,
  LobbyPhase,
  type LobbyState,
  type LobbySummary,
} from "@/proto/minigames/v2/lobby_pb";
import { RequestError } from "@/net/request-error";
import type { ServerPayload } from "@/net/session";
import { playerLabels, sameIdentity } from "@/util/players";

import { useConnectionStore } from "./connection";
import { useToastStore } from "./toasts";

const MAX_CHAT_MESSAGES = 200;

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
  /** The lobby left last: its late updates must not bring it back. */
  let leftLobbyId: string | null = null;
  /** A leave sent while the connection was down; the server puts us back on resume otherwise. */
  const isLeaving = ref(false);

  const isHost = computed(() => sameIdentity(current.value?.host, connection.identity));
  const mySeatIndex = computed(() =>
    current.value
      ? current.value.seats.findIndex((seat) => sameIdentity(seat.occupant, connection.identity))
      : -1,
  );
  /** Seated players first, then observers, so seat labels do not change when someone watches. */
  const members = computed(() => [
    ...(current.value?.seats.map((seat) => seat.occupant) ?? []),
    ...(current.value?.observers ?? []),
  ]);
  const memberLabels = computed(() => playerLabels(members.value));

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
      /** Later sign-ins of this session are reconnects. */
      let reconnecting = false;
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
        session.events.on("signedIn", ({ resumed }) => {
          if (isLeaving.value) {
            if (resumed) void sendLeave();
            else isLeaving.value = false;
          } else if (!resumed && current.value) {
            current.value = null;
            lobbyChat.value = [];
            // The first sign-in of a new session (another account, say) is no reconnect.
            if (connection.lostToRestart()) {
              toasts.show("The server restarted for an update, and the lobby was closed");
            } else if (reconnecting) {
              toasts.show("You were away too long and lost your place in the lobby");
            }
          }
          reconnecting = true;
          void refresh();
        }),
      );
    },
    { immediate: true },
  );

  /** Ignores late updates for a lobby we already left. */
  function accept(state: LobbyState): void {
    const known = current.value;
    if (known && known.lobbyId !== state.lobbyId) return;
    if (!known && (isLeaving.value || state.lobbyId === leftLobbyId)) return;
    const hadVoted = known?.playAgainVotes.some((voter) => sameIdentity(voter, connection.identity));
    const stillVoted = state.playAgainVotes.some((voter) => sameIdentity(voter, connection.identity));
    // The server drops the votes when the players or the settings change after a match.
    if (hadVoted && !stillVoted && state.phase === LobbyPhase.POST_MATCH) {
      toasts.show("The players or settings changed: vote again to play");
    }
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
    if (reply.case === "lobbyState") enter(reply.value);
  }

  async function join(lobbyId: string, asObserver: boolean): Promise<void> {
    const reply = await call({ case: "joinLobby", value: { lobbyId, asObserver } });
    if (reply.case === "lobbyState") enter(reply.value);
  }

  function enter(state: LobbyState): void {
    current.value = state;
    lobbyChat.value = [];
    leftLobbyId = null;
    isLeaving.value = false;
  }

  async function leave(): Promise<void> {
    leftLobbyId = current.value?.lobbyId ?? leftLobbyId;
    current.value = null;
    lobbyChat.value = [];
    isLeaving.value = true;
    await sendLeave();
    await refresh();
  }

  /** Leaves on the server; a request the server never got is sent again after the reconnect. */
  async function sendLeave(): Promise<void> {
    try {
      await connection.requireSession().request({ case: "leaveLobby", value: {} });
      isLeaving.value = false;
    } catch (error) {
      if (error instanceof RequestError && error.failure === "refused") isLeaving.value = false;
    }
  }

  const setReady = (ready: boolean) => call({ case: "setReady", value: { ready } });
  const addBot = (botKind: BotKind) => call({ case: "addBot", value: { botKind } });
  const removeSeat = (seat: number) => call({ case: "removeSeat", value: { seat } });
  const kick = (playerId: string) => call({ case: "kickMember", value: { playerId } });
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
    members,
    memberLabels,
    inMatch,
    isLeaving,
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
