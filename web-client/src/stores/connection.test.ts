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
