import { create } from "@bufbuild/protobuf";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";
import { Presence } from "@/proto/minigames/v2/social_pb";
import { useConnectionStore } from "@/stores/connection";
import { useFriendsStore } from "@/stores/friends";
import { useLobbyStore } from "@/stores/lobby";

import OnlineView from "./OnlineView.vue";

vi.mock("@/wasm/core", () => ({
  gameCatalog: () => [],
  defaultSettings: () => ({}),
  settingsProblem: () => null,
}));
vi.mock("@/games", () => ({ gamePlugins: [], pluginBySettings: () => undefined }));

const me = { kind: IdentityKind.GOOGLE, id: "g_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.1", displayName: "Ada" };
const guest = { kind: IdentityKind.GUEST, id: "guest-1", displayName: "Ada" };

/** Signed in as `identity`; the server's later answers are up to the test. */
async function signedIn(identity: typeof me | typeof guest): Promise<FakeSocket> {
  useLobbyStore();
  useFriendsStore();
  const connection = useConnectionStore();
  if (identity.kind === IdentityKind.GOOGLE) connection.signInWithGoogle("id-token");
  else connection.signInAsGuest(identity.displayName);
  const socket = FakeSocket.opened[0];
  if (!socket) throw new Error("no socket was opened");
  // The stores subscribe to the new session in their watchers: let those run first.
  await flushPromises();
  socket.accept();
  socket.push(
    create(ServerMessageSchema, {
      requestId: socket.lastRequestId(),
      message: { case: "welcome", value: { identity, resumeToken: "resume", resumed: true } },
    }),
  );
  await vi.waitFor(() => expect(connection.isSignedIn).toBe(true));
  return socket;
}

const bob = { kind: IdentityKind.GOOGLE, id: "g_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", displayName: "Bob" };
const cleo = { kind: IdentityKind.GOOGLE, id: "g_cccccccccccccccccccccccccccccccc", displayName: "Cleo" };

/** Bob online, Cleo offline, and Dan asking to be a friend. */
function pushFriends(socket: FakeSocket): void {
  socket.push(
    create(ServerMessageSchema, {
      message: {
        case: "friendList",
        value: {
          friends: [
            { identity: cleo, presence: Presence.OFFLINE },
            { identity: bob, presence: Presence.ONLINE },
          ],
          incomingRequests: [
            { kind: IdentityKind.GOOGLE, id: "g_dddddddddddddddddddddddddddddddd", displayName: "Dan" },
          ],
        },
      },
    }),
  );
}

async function mountAt(path: string) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/online", name: "online", component: OnlineView },
      { path: "/lobby", name: "lobby", component: { template: "<p>lobby</p>" } },
      { path: "/friends", component: { template: "<p>friends</p>" } },
      { path: "/", component: { template: "<p>home</p>" } },
    ],
  });
  await router.push(path);
  const wrapper = mount(OnlineView, { global: { plugins: [router] } });
  await flushPromises();
  return { router, wrapper };
}

describe("OnlineView", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("offers to open the lobby the player is in instead of joining it again", async () => {
    const socket = await signedIn(me);
    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "lobbyState",
          value: {
            lobbyId: "lobby-1",
            name: "Ada's game",
            host: me,
            maxSeats: 2,
            seats: [{ occupant: me }, {}],
          },
        },
      }),
    );
    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "lobbyList",
          value: {
            lobbies: [{ lobbyId: "lobby-1", name: "Ada's game", host: me, seatsTaken: 1, maxSeats: 2 }],
          },
        },
      }),
    );
    await vi.waitFor(() => expect(useLobbyStore().lobbies).toHaveLength(1));
    const { router, wrapper } = await mountAt("/online");

    const buttons = wrapper.findAll("button").map((button) => button.text());
    expect(buttons).not.toContain("Join");
    expect(buttons).not.toContain("+ New lobby");
    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Open")
      ?.trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.name).toBe("lobby");
  });

  it("lists the friends with their presence, the online ones first, beside the lobbies", async () => {
    const socket = await signedIn(me);
    pushFriends(socket);

    const { wrapper } = await mountAt("/online");

    const panel = wrapper.find('[data-testid="friends-panel"]');
    expect(panel.exists()).toBe(true);
    const rows = panel.findAll("li").map((row) => row.text());
    expect(rows).toHaveLength(2);
    expect(rows[0]).toContain("Bob");
    expect(rows[0]).toContain("Online");
    expect(rows[1]).toContain("Cleo");
    expect(rows[1]).toContain("Offline");
    // Managing friends (the request waiting among it) is on the Friends screen.
    expect(panel.find('a[href="/friends"]').text()).toContain("1 request");
    // Not in a lobby: nobody to invite.
    expect(panel.findAll("button")).toHaveLength(0);
  });

  it("shows no friends to a guest", async () => {
    await signedIn(guest);

    const { wrapper } = await mountAt("/online");

    expect(wrapper.find('[data-testid="friends-panel"]').exists()).toBe(false);
    expect(wrapper.find('a[href="/friends"]').exists()).toBe(false);
  });

  it("offers to invite an online friend from the lobby the player is in", async () => {
    const socket = await signedIn(me);
    pushFriends(socket);
    socket.push(
      create(ServerMessageSchema, {
        message: {
          case: "lobbyState",
          value: {
            lobbyId: "lobby-1",
            name: "Ada's game",
            host: me,
            maxSeats: 2,
            seats: [{ occupant: me }, {}],
          },
        },
      }),
    );
    await vi.waitFor(() => expect(useLobbyStore().current).not.toBeNull());

    const { wrapper } = await mountAt("/online");

    const invites = wrapper.find('[data-testid="friends-panel"]').findAll("button");
    expect(invites.map((button) => button.text())).toEqual(["Invite"]);
    await invites[0]?.trigger("click");
    const last = socket.sent.at(-1) ?? "";
    expect(last).toContain("inviteFriend");
    expect(last).toContain(bob.id);
  });
});
