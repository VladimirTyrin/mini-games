import { create } from "@bufbuild/protobuf";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ErrorCode, type ServerMessage, ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { type ConnectionState, Session } from "./session";
import { FakeSocket } from "./testing/fake-socket";

function welcome(requestId: number): ServerMessage {
  return create(ServerMessageSchema, {
    requestId,
    message: {
      case: "welcome",
      value: { identity: { displayName: "Alice" }, resumeToken: "resume" },
    },
  });
}

async function signedInSession(): Promise<{
  session: Session;
  socket: FakeSocket;
  states: ConnectionState[];
}> {
  const session = new Session("ws://test/ws", { kind: "guest", displayName: "Alice" });
  const states: ConnectionState[] = [];
  session.events.on("state", (state) => states.push(state));
  session.connect();
  const socket = FakeSocket.opened[0];
  if (!socket) throw new Error("no socket opened");
  socket.accept();
  socket.push(welcome(socket.lastRequestId()));
  await vi.waitFor(() => expect(session.state).toBe("signed-in"));
  return { session, socket, states };
}

describe("Session", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("reconnects after the connection drops", async () => {
    const { socket } = await signedInSession();

    socket.close();
    await vi.advanceTimersByTimeAsync(60_000);

    expect(FakeSocket.opened).toHaveLength(2);
  });

  it("stays away after the account signed in elsewhere", async () => {
    const { session, socket, states } = await signedInSession();

    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "error",
          value: { code: ErrorCode.SIGNED_IN_ELSEWHERE, message: "signed in elsewhere" },
        },
      }),
    );
    socket.close();
    await vi.advanceTimersByTimeAsync(60_000);

    expect(session.state).toBe("replaced");
    expect(states.at(-1)).toBe("replaced");
    expect(FakeSocket.opened).toHaveLength(1);
  });
});
