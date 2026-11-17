import { create, fromJsonString } from "@bufbuild/protobuf";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import {
  type ClientMessage,
  ClientMessageSchema,
  ErrorCode,
  ServerMessageSchema,
} from "@/proto/minigames/v2/protocol_pb";
import { InviteState } from "@/proto/minigames/v2/social_pb";

import { useConnectionStore } from "./connection";
import { useFriendsStore } from "./friends";
import { useLobbyStore } from "./lobby";

vi.mock("@/platform/push-notifications", () => ({
  canNotifyWithoutAsking: vi.fn().mockResolvedValue(true),
  pushAvailability: vi.fn().mockReturnValue("available"),
  registerForPush: vi.fn().mockResolvedValue({ target: { case: "fcmToken", value: "token" } }),
}));

const me = { kind: IdentityKind.GOOGLE, id: "g_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.1", displayName: "Ada" };
const bob = { kind: IdentityKind.GOOGLE, id: "g_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", displayName: "Bob" };

function sent(socket: FakeSocket): ClientMessage[] {
  return socket.sent.map((text) => fromJsonString(ClientMessageSchema, text));
}

/** Answers each request as it is sent: what the store expects, an Ack by default. */
function answerRequests(
  socket: FakeSocket,
  refuse: (message: ClientMessage) => ErrorCode | null = () => null,
) {
  const send = socket.send.bind(socket);
  socket.send = (data: string) => {
    send(data);
    const request = fromJsonString(ClientMessageSchema, data);
    const code = refuse(request);
    queueMicrotask(() =>
      socket.push(
        create(ServerMessageSchema, {
          requestId: request.requestId,
          message:
            code !== null
              ? { case: "error", value: { code, message: "refused" } }
              : request.message.case === "listFriends"
                ? { case: "friendList", value: {} }
                : { case: "ack", value: {} },
        }),
      ),
    );
  };
}

/** Signed in with Google, the server answering every later request (see `answerRequests`). */
async function signedIn(refuse?: (message: ClientMessage) => ErrorCode | null): Promise<FakeSocket> {
  useConnectionStore().signInWithGoogle("id-token");
  const socket = FakeSocket.opened[0];
  if (!socket) throw new Error("no socket was opened");
  socket.accept();
  const helloId = socket.lastRequestId();
  answerRequests(socket, refuse);
  socket.push(
    create(ServerMessageSchema, {
      requestId: helloId,
      message: { case: "welcome", value: { identity: me, resumeToken: "resume", sessionToken: "session" } },
    }),
  );
  await vi.waitFor(() => expect(useConnectionStore().isSignedIn).toBe(true));
  return socket;
}

describe("friends store", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("registers the device for push before answering an invitation opened from a notification", async () => {
    const friends = useFriendsStore();
    void friends.openInvite("invite-1", "lobby-1");

    const socket = await signedIn();

    await vi.waitFor(() => expect(sent(socket).some((m) => m.message.case === "answerInvite")).toBe(true));
    const cases = sent(socket).map((message) => message.message.case);
    expect(cases.indexOf("registerPushTarget")).toBeGreaterThan(-1);
    expect(cases.indexOf("registerPushTarget")).toBeLessThan(cases.indexOf("answerInvite"));
  });

  it("counts an invitation down from the time left, whatever the device clock says", async () => {
    const friends = useFriendsStore();
    const socket = await signedIn();

    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "gameInvite",
          // The server's clock says it ends now; this device's clock is a minute fast.
          value: {
            inviteId: "invite-1",
            from: bob,
            expiresAtUnixMs: BigInt(Date.now() - 60_000),
            expiresInMs: 90_000,
          },
        },
      }),
    );

    await vi.waitFor(() => expect(friends.incomingInvites).toHaveLength(1));
    expect(friends.deadlineOf("invite-1") - Date.now()).toBeGreaterThan(80_000);
  });

  it("stops waiting for an invitation that closed while its news was lost", async () => {
    const friends = useFriendsStore();
    const socket = await signedIn((message) =>
      message.message.case === "cancelInvite" ? ErrorCode.INVITE_CLOSED : null,
    );
    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "inviteUpdate",
          value: { inviteId: "invite-1", friend: bob, state: InviteState.PENDING, expiresInMs: 90_000 },
        },
      }),
    );
    await vi.waitFor(() => expect(friends.sentInvites.size).toBe(1));

    await friends.cancelInvite("invite-1");

    expect(friends.sentInvites.get("invite-1")?.state).toBe(InviteState.EXPIRED);
  });

  it("forgets an accepted invitation once the friend is in the lobby, so leaving shows no stale news", async () => {
    const friends = useFriendsStore();
    const socket = await signedIn();
    const bobsDevice = { ...bob, id: `${bob.id}.2` };
    const lobbyWith = (members: (typeof me)[]) =>
      create(ServerMessageSchema, {
        message: {
          case: "lobbyState",
          value: {
            lobbyId: "lobby-1",
            host: me,
            maxSeats: 2,
            seats: members.map((occupant) => ({ occupant })),
          },
        },
      });
    socket.push(lobbyWith([me]));
    await vi.waitFor(() => expect(useLobbyStore().current?.lobbyId).toBe("lobby-1"));
    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "inviteUpdate",
          value: { inviteId: "invite-1", friend: bob, state: InviteState.ACCEPTED, expiresInMs: 60_000 },
        },
      }),
    );
    await vi.waitFor(() => expect(friends.sentInvites.size).toBe(1));

    socket.push(lobbyWith([me, bobsDevice]));
    await vi.waitFor(() => expect(friends.sentInvites.size).toBe(0));
    socket.push(lobbyWith([me]));

    await vi.waitFor(() => expect(useLobbyStore().current?.seats).toHaveLength(1));
    expect(friends.sentInvites.size).toBe(0);
  });
});
