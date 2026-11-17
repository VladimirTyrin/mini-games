import { create } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { MatchSource } from "@/match/match-source";
import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { LobbyStateSchema } from "@/proto/minigames/v2/lobby_pb";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";
import { useMatchStore } from "./match";
import { useToastStore } from "./toasts";

const me = { kind: IdentityKind.GUEST, id: "u_me", displayName: "Sam" };

function openedSocket(index: number): FakeSocket {
  const socket = FakeSocket.opened[index];
  if (!socket) throw new Error(`socket ${index} was not opened`);
  return socket;
}

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

/** Signed in and seated in a running online match; returns the socket. */
async function inOnlineMatch(): Promise<FakeSocket> {
  const match = useMatchStore();
  useConnectionStore().signInAsGuest("Sam");
  const socket = openedSocket(0);
  await welcomeOn(socket, false);
  socket.push(
    create(ServerMessageSchema, {
      message: { case: "matchStarted", value: { matchId: "m1", seats: [{ slot: 0, player: me }] } },
    }),
  );
  expect(match.controller?.source.kind).toBe("online");
  return socket;
}

// The real ones need the WASM core; the store only wires them together.
vi.mock("@/match/replay-match-source", () => ({
  ReplayMatchSource: class {
    readonly kind = "replay";
    start(): void {}
    leave(): void {}
  },
}));
vi.mock("@/match/match-controller", async () => {
  const { shallowRef } = await import("vue");
  return {
    MatchController: class {
      readonly incompatible = shallowRef(false);
      constructor(readonly source: unknown) {}
      dispose(): void {}
    },
  };
});
vi.mock("@/games", () => ({ pluginBySettings: () => undefined }));

describe("match store", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("keeps the replay it starts, so the replay controls show", () => {
    const match = useMatchStore();
    match.startReplay(new Uint8Array([1]));
    expect(match.replay).not.toBeNull();
    expect(match.controller?.source).toBe(match.replay);
  });

  it("forgets the replay when the match closes", () => {
    const match = useMatchStore();
    match.startReplay(new Uint8Array([1]));
    match.close();
    expect(match.replay).toBeNull();
  });

  it("moves a running online match to the new session after 'Continue here'", async () => {
    await inOnlineMatch();
    const match = useMatchStore();
    const source = match.controller?.source as MatchSource;
    const ticks: bigint[] = [];
    source.on("frame", (frame) => ticks.push(frame.tick));

    useConnectionStore().continueHere();
    const fresh = openedSocket(1);
    await welcomeOn(fresh, true);
    fresh.push(create(ServerMessageSchema, { message: { case: "frame", value: { tick: 7n } } }));

    expect(match.controller?.source).toBe(source);
    expect(ticks).toEqual([7n]);
  });

  it("ends the online match when the server no longer kept the seat after a reconnect", async () => {
    vi.useFakeTimers();
    try {
      const socket = await inOnlineMatch();
      const match = useMatchStore();

      socket.close();
      await vi.advanceTimersByTimeAsync(60_000);
      await welcomeOn(openedSocket(1), false);

      expect(match.controller).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("says the match was stopped by a server restart instead of closing it silently", async () => {
    vi.useFakeTimers();
    try {
      const socket = await inOnlineMatch();
      const match = useMatchStore();

      socket.push(create(ServerMessageSchema, { message: { case: "shuttingDown", value: {} } }));
      socket.close();
      await vi.advanceTimersByTimeAsync(5_000);
      await welcomeOn(openedSocket(1), false);

      expect(match.controller).toBeNull();
      expect(useToastStore().toasts.map((toast) => toast.text)).toContain(
        "The server restarted for an update, and the match was stopped",
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it("holds a new version back in an online lobby, not only in a match", () => {
    const match = useMatchStore();
    expect(match.holdsUpdates).toBe(false);

    useLobbyStore().current = create(LobbyStateSchema, { lobbyId: "lobby" });

    expect(match.holdsUpdates).toBe(true);
  });

  it("lets a new version in when the match on screen cannot be followed by this build", async () => {
    await inOnlineMatch();
    const match = useMatchStore();
    expect(match.holdsUpdates).toBe(true);

    if (match.controller) match.controller.incompatible.value = true;

    expect(match.holdsUpdates).toBe(false);
  });
});
