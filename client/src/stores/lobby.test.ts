import { create } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { LobbyPhase } from "@/proto/minigames/v2/lobby_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";
import { useToastStore } from "./toasts";

const otherSam = { kind: IdentityKind.GUEST, id: "u_other", displayName: "Sam" };
const me = { kind: IdentityKind.GUEST, id: "u_me", displayName: "Sam" };

function openedSocket(index: number): FakeSocket {
  const socket = FakeSocket.opened[index];
  if (!socket) throw new Error(`socket ${index} was not opened`);
  return socket;
}

/** Plays the server's welcome on the socket's hello. */
async function welcomeOn(socket: FakeSocket, resumed: boolean): Promise<void> {
  socket.accept();
  socket.push(
    create(ServerMessageSchema, {
      requestId: socket.lastRequestId(),
      message: { case: "welcome", value: { identity: me, resumeToken: "resume", resumed } },
    }),
  );
  await vi.waitFor(() => expect(useConnectionStore().isSignedIn).toBe(true));
}

function pushLobbyState(socket: FakeSocket, lobbyId: string): void {
  socket.push(
    create(ServerMessageSchema, {
      message: {
        case: "lobbyState",
        value: { lobbyId, host: me, maxSeats: 2, seats: [{ occupant: me }, {}] },
      },
    }),
  );
}

/** Signed in as a guest and seated in "lobby"; returns the socket. */
async function inLobby(): Promise<FakeSocket> {
  useLobbyStore();
  useConnectionStore().signInAsGuest("Sam");
  const socket = openedSocket(0);
  await welcomeOn(socket, false);
  pushLobbyState(socket, "lobby");
  await vi.waitFor(() => expect(useLobbyStore().current?.lobbyId).toBe("lobby"));
  return socket;
}

/** Answers the last request with an Ack. */
function ack(socket: FakeSocket): void {
  socket.push(
    create(ServerMessageSchema, { requestId: socket.lastRequestId(), message: { case: "ack", value: {} } }),
  );
}

describe("lobby store", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("finds the viewer's seat by id when another player has the same name", async () => {
    const connection = useConnectionStore();
    const lobby = useLobbyStore();
    connection.signInAsGuest("Sam");
    const socket = FakeSocket.opened[0];
    if (!socket) throw new Error("no socket opened");
    socket.accept();
    socket.push(
      create(ServerMessageSchema, {
        requestId: socket.lastRequestId(),
        message: { case: "welcome", value: { identity: me, resumeToken: "resume" } },
      }),
    );
    await vi.waitFor(() => expect(connection.isSignedIn).toBe(true));

    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "lobbyState",
          value: {
            lobbyId: "lobby",
            host: otherSam,
            maxSeats: 2,
            seats: [{ occupant: otherSam }, { occupant: me }],
          },
        },
      }),
    );

    await vi.waitFor(() => expect(lobby.current?.lobbyId).toBe("lobby"));
    expect(lobby.isHost).toBe(false);
    expect(lobby.mySeatIndex).toBe(1);
    expect(lobby.memberLabels.get("u_other")).toBe("Sam");
    expect(lobby.memberLabels.get("u_me")).toBe("Sam · 2");
  });

  it("ignores a late update of the lobby it just left", async () => {
    const socket = await inLobby();
    const lobby = useLobbyStore();

    const leaving = lobby.leave();
    expect(socket.lastSent().message.case).toBe("leaveLobby");
    pushLobbyState(socket, "lobby");
    ack(socket);
    await vi.waitFor(() => expect(socket.lastSent().message.case).toBe("listLobbies"));
    ack(socket);
    await leaving;

    expect(lobby.current).toBeNull();
  });

  it("follows this session into another lobby when the server says it is there", async () => {
    const socket = await inLobby();
    const lobby = useLobbyStore();

    // Continuing a session another device left in a different lobby.
    pushLobbyState(socket, "elsewhere");

    await vi.waitFor(() => expect(lobby.current?.lobbyId).toBe("elsewhere"));
  });

  it("finishes a leave sent while offline once the connection is back", async () => {
    vi.useFakeTimers();
    try {
      const socket = await inLobby();
      const lobby = useLobbyStore();
      socket.close();

      await lobby.leave();
      await vi.advanceTimersByTimeAsync(60_000);
      const again = openedSocket(1);
      await welcomeOn(again, true);
      // On resume the server puts the member back into the lobby first.
      pushLobbyState(again, "lobby");

      const sent = again.sent.map((raw, index) => (index === 0 ? "hello" : raw));
      expect(sent.some((raw) => raw.includes("leaveLobby"))).toBe(true);
      expect(lobby.current).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("drops the lobby when the server no longer kept the seat after a reconnect", async () => {
    vi.useFakeTimers();
    try {
      const socket = await inLobby();
      const lobby = useLobbyStore();

      socket.close();
      await vi.advanceTimersByTimeAsync(60_000);
      await welcomeOn(openedSocket(1), false);

      expect(lobby.current).toBeNull();
      expect(useToastStore().toasts.map((toast) => toast.text)).toContain(
        "You are no longer in the lobby you were in",
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it("tells the player when a play-again vote was dropped because the table changed", async () => {
    const socket = await inLobby();
    const postMatch = (votes: (typeof me)[]) =>
      create(ServerMessageSchema, {
        message: {
          case: "lobbyState",
          value: {
            lobbyId: "lobby",
            host: me,
            maxSeats: 2,
            seats: [{ occupant: me }],
            phase: LobbyPhase.POST_MATCH,
            playAgainVotes: votes,
          },
        },
      });

    socket.push(postMatch([me]));
    socket.push(postMatch([]));

    await vi.waitFor(() =>
      expect(useToastStore().toasts.map((toast) => toast.text)).toContain(
        "The players or settings changed: vote again to play",
      ),
    );
  });
});
