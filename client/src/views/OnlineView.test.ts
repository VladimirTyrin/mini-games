import { create } from "@bufbuild/protobuf";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

import { FakeSocket } from "@/net/testing/fake-socket";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";
import { useConnectionStore } from "@/stores/connection";
import { useLobbyStore } from "@/stores/lobby";

import OnlineView from "./OnlineView.vue";

vi.mock("@/wasm/core", () => ({
  gameCatalog: () => [],
  defaultSettings: () => ({}),
  settingsProblem: () => null,
}));
vi.mock("@/games", () => ({ gamePlugins: [], pluginBySettings: () => undefined }));

const me = { kind: IdentityKind.GOOGLE, id: "g_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.1", displayName: "Ada" };

describe("OnlineView", () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => vi.unstubAllGlobals());

  it("offers to open the lobby the player is in instead of joining it again", async () => {
    useLobbyStore();
    useConnectionStore().signInWithGoogle("id-token");
    const socket = FakeSocket.opened[0];
    if (!socket) throw new Error("no socket was opened");
    // The stores subscribe to the new session in their watchers: let those run first.
    await flushPromises();
    socket.accept();
    socket.push(
      create(ServerMessageSchema, {
        requestId: socket.lastRequestId(),
        message: { case: "welcome", value: { identity: me, resumeToken: "resume", resumed: true } },
      }),
    );
    await vi.waitFor(() => expect(useConnectionStore().isSignedIn).toBe(true));
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
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: "/online", name: "online", component: OnlineView },
        { path: "/lobby", name: "lobby", component: { template: "<p>lobby</p>" } },
        { path: "/friends", component: { template: "<p>friends</p>" } },
        { path: "/", component: { template: "<p>home</p>" } },
      ],
    });
    await router.push("/online");

    const wrapper = mount(OnlineView, { global: { plugins: [router] } });
    await flushPromises();

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
});
