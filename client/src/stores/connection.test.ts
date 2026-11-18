import { create } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeSocket } from "@/net/testing/fake-socket";
import { ErrorCode, ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { useConnectionStore } from "./connection";
import { usePrefsStore } from "./prefs";
import { useToastStore } from "./toasts";

function openedSocket(index: number): FakeSocket {
  const socket = FakeSocket.opened[index];
  if (!socket) throw new Error(`socket ${index} was not opened`);
  return socket;
}

describe("connection store", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("forgets a stored session the server refuses and asks for a new sign-in", async () => {
    const prefs = usePrefsStore();
    prefs.sessionToken = "stale";
    const connection = useConnectionStore();
    const toasts = useToastStore();

    connection.resumeStoredSession();
    const socket = openedSocket(0);
    socket.accept();
    expect(socket.lastSent().message.case).toBe("hello");
    socket.push(
      create(ServerMessageSchema, {
        requestId: socket.lastRequestId(),
        message: {
          case: "error",
          value: { code: ErrorCode.SIGN_IN_FAILED, message: "invalid session token" },
        },
      }),
    );

    await vi.waitFor(() => expect(prefs.sessionToken).toBe(""));
    expect(toasts.toasts.map((toast) => toast.text)).toEqual([
      "Your session has expired. Please sign in again.",
    ]);
  });

  it("forgets the session a Google sign-in returned once the server refuses it on a reconnect", async () => {
    vi.useFakeTimers();
    const prefs = usePrefsStore();
    const connection = useConnectionStore();
    const toasts = useToastStore();

    connection.signInWithGoogle("google-id-token");
    const first = openedSocket(0);
    first.accept();
    first.push(
      create(ServerMessageSchema, {
        requestId: first.lastRequestId(),
        message: {
          case: "welcome",
          value: { identity: { displayName: "Alice" }, resumeToken: "resume", sessionToken: "session-token" },
        },
      }),
    );
    await vi.waitFor(() => expect(prefs.sessionToken).toBe("session-token"));
    first.close();
    await vi.advanceTimersByTimeAsync(60_000);
    const second = openedSocket(1);
    second.accept();
    second.push(
      create(ServerMessageSchema, {
        requestId: second.lastRequestId(),
        message: {
          case: "error",
          value: { code: ErrorCode.SIGN_IN_FAILED, message: "invalid session token" },
        },
      }),
    );

    await vi.waitFor(() => expect(prefs.sessionToken).toBe(""));
    expect(toasts.toasts.map((toast) => toast.text)).toEqual([
      "Your session has expired. Please sign in again.",
    ]);
    vi.useRealTimers();
  });

  it("signs in again with the stored session when continuing on this device", () => {
    const prefs = usePrefsStore();
    prefs.sessionToken = "mine";
    const connection = useConnectionStore();

    connection.continueHere();
    const socket = openedSocket(0);
    socket.accept();

    const hello = socket.lastSent().message;
    expect(hello.case === "hello" && hello.value.credentials.case).toBe("session");
  });
});
