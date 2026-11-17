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

  it("drops a connection that stops answering pings and reconnects", async () => {
    const { socket, states } = await signedInSession();

    // The first ping goes out at once; the server never answers it.
    await vi.advanceTimersByTimeAsync(10_000);

    expect(socket.readyState).toBe(3);
    expect(states.at(-1)).toBe("reconnecting");
    await vi.advanceTimersByTimeAsync(30_000);
    expect(FakeSocket.opened).toHaveLength(2);
  });

  it("reports the round trip of a ping", async () => {
    const session = new Session("ws://test/ws", { kind: "guest", displayName: "Alice" });
    const latencies: number[] = [];
    session.events.on("latency", (ms) => latencies.push(ms));
    session.connect();
    const socket = FakeSocket.opened[0];
    if (!socket) throw new Error("no socket opened");
    socket.accept();
    socket.push(welcome(socket.lastRequestId()));
    // Signed in without time passing: the first ping goes out now.
    await vi.advanceTimersByTimeAsync(0);
    const ping = socket.lastSent();
    expect(ping.message.case).toBe("ping");

    await vi.advanceTimersByTimeAsync(42);
    socket.push(
      create(ServerMessageSchema, { requestId: ping.requestId, message: { case: "pong", value: {} } }),
    );
    await vi.advanceTimersByTimeAsync(0);

    expect(latencies).toEqual([42]);
  });

  it("retries a sign-in the server did not answer, without reporting a refusal", async () => {
    const session = new Session("ws://test/ws", { kind: "session", token: "kept" });
    const refusals: unknown[] = [];
    session.events.on("refused", (error) => refusals.push(error));
    session.connect();
    FakeSocket.opened[0]?.accept();

    await vi.advanceTimersByTimeAsync(10_000);

    expect(refusals).toEqual([]);
    expect(session.state).toBe("reconnecting");
    await vi.advanceTimersByTimeAsync(30_000);
    const retry = FakeSocket.opened[1];
    if (!retry) throw new Error("no retry");
    retry.accept();
    const hello = retry.lastSent().message;
    expect(hello.case === "hello" && hello.value.credentials).toEqual({
      case: "session",
      value: expect.objectContaining({ token: "kept" }),
    });
  });

  it("tries again when the server cannot check a sign-in just now", async () => {
    const session = new Session("ws://test/ws", { kind: "session", token: "kept" });
    const refusals: unknown[] = [];
    session.events.on("refused", (error) => refusals.push(error));
    session.connect();
    const first = FakeSocket.opened[0];
    if (!first) throw new Error("no socket opened");
    first.accept();

    first.push(
      create(ServerMessageSchema, {
        requestId: first.lastRequestId(),
        message: { case: "error", value: { code: ErrorCode.RATE_LIMITED, message: "retry later" } },
      }),
    );
    await vi.advanceTimersByTimeAsync(30_000);

    expect(refusals).toEqual([]);
    expect(FakeSocket.opened.length).toBeGreaterThan(1);
  });
});
