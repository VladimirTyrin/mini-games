import { create } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

import { useConnectionStore } from "./connection";
import { TEST_PUSH_DELAY_SECONDS, useNotificationsStore } from "./notifications";

const register = vi.hoisted(() => vi.fn());
vi.mock("@/platform/push-notifications", () => ({
  registerForPush: register,
  onNotification: vi.fn(),
}));

const me = { kind: IdentityKind.GUEST, id: "u_me", displayName: "Sam" };

async function signedIn(webPushPublicKey: string): Promise<FakeSocket> {
  useConnectionStore().signInAsGuest("Sam");
  const socket = FakeSocket.opened[0];
  if (!socket) throw new Error("no socket");
  socket.accept();
  socket.push(
    create(ServerMessageSchema, {
      requestId: socket.lastRequestId(),
      message: {
        case: "welcome",
        value: { identity: me, resumeToken: "resume", serverInfo: { webPushPublicKey } },
      },
    }),
  );
  await vi.waitFor(() => expect(useConnectionStore().isSignedIn).toBe(true));
  return socket;
}

describe("notifications store", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    register.mockReset();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("asks the server for a test push to this device with the server's key", async () => {
    const socket = await signedIn("server-key");
    const target = { target: { case: "fcmToken" as const, value: "token" } };
    register.mockResolvedValue(target);
    const notifications = useNotificationsStore();

    const sending = notifications.sendTest();
    await vi.waitFor(() => expect(socket.lastSent().message.case).toBe("requestTestPush"));
    socket.push(
      create(ServerMessageSchema, { requestId: socket.lastRequestId(), message: { case: "ack", value: {} } }),
    );
    await sending;

    expect(register).toHaveBeenCalledWith("server-key");
    const sent = socket.lastSent().message;
    expect(sent.case === "requestTestPush" && sent.value.delaySeconds).toBe(TEST_PUSH_DELAY_SECONDS);
    expect(sent.case === "requestTestPush" && sent.value.target?.target.value).toBe("token");
    expect(notifications.testStatus.kind).toBe("scheduled");
  });

  it("says why the device could not be set up", async () => {
    await signedIn("server-key");
    register.mockRejectedValue(new Error("Notifications are turned off for this app"));
    const notifications = useNotificationsStore();

    await notifications.sendTest();

    expect(notifications.testStatus).toEqual({
      kind: "failed",
      message: "Notifications are turned off for this app",
    });
  });
});
