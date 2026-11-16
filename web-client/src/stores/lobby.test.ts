import { create } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";

const otherSam = { kind: IdentityKind.GUEST, id: "u_other", displayName: "Sam" };
const me = { kind: IdentityKind.GUEST, id: "u_me", displayName: "Sam" };

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
});
