import { create } from "@bufbuild/protobuf";
import { type VueWrapper, flushPromises, shallowMount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

import type { ReplayMatchSource } from "@/match/replay-match-source";
import { GameSettingsSchema } from "@/proto/minigames/v2/game_pb";
import { useMatchStore } from "@/stores/match";

import MatchView from "./MatchView.vue";

const screen = vi.hoisted(() => ({ keptOn: false }));
vi.mock("@/platform/keep-awake", () => ({
  setKeepScreenOn: (keepScreenOn: boolean) => {
    screen.keptOn = keepScreenOn;
    return Promise.resolve();
  },
}));
// The real ones need the WASM core; this view only reads their state.
vi.mock("@/match/replay-match-source", () => ({
  ReplayMatchSource: class {
    readonly kind = "replay";
    readonly header = {};
    readonly speed = 100;
    isPlaying = false;
    private readonly handlers = new Set<() => void>();
    onProgress(handler: () => void): () => void {
      this.handlers.add(handler);
      return () => this.handlers.delete(handler);
    }
    start(): void {
      this.play();
    }
    play(): void {
      this.isPlaying = true;
      this.handlers.forEach((handler) => handler());
    }
    pause(): void {
      this.isPlaying = false;
      this.handlers.forEach((handler) => handler());
    }
    leave(): void {
      this.pause();
    }
  },
}));
/** The player's own pauses of a local match, as they reach the source. */
const playerPauses = vi.hoisted(() => [] as string[]);
vi.mock("@/local/local-match-source", () => ({
  LocalMatchSource: class {
    readonly kind = "local";
    constructor(readonly header: unknown) {}
    start(): void {}
    leave(): void {}
    setPaused(reason: string, paused: boolean, countdownMs = 0): void {
      if (reason === "player") playerPauses.push(paused ? "paused" : `resumed in ${countdownMs} ms`);
    }
  },
}));
vi.mock("@/match/match-controller", async () => {
  const { shallowRef } = await import("vue");
  return {
    MatchController: class {
      readonly incompatible = shallowRef(false);
      readonly ended = shallowRef(false);
      readonly current = shallowRef(null);
      constructor(readonly source: { kind: string }) {}
      /** Local matches here are real-time ones (pausable), replays turn-based. */
      get gameTickMs(): number {
        return this.source.kind === "local" ? 100 : 0;
      }
      applyCatchUp(): void {}
      dispose(): void {}
    },
  };
});
vi.mock("@/games", () => ({ pluginBySettings: () => undefined }));

async function watchingReplay(): Promise<ReplayMatchSource> {
  const match = useMatchStore();
  match.startReplay(new Uint8Array([1]));
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/:any(.*)", component: MatchView }],
  });
  shallowMount(MatchView, { global: { plugins: [router] } });
  await flushPromises();
  const replay = match.replay;
  if (!replay) throw new Error("no replay started");
  return replay;
}

async function playingLocally(): Promise<VueWrapper> {
  useMatchStore().startLocal(
    create(GameSettingsSchema, { game: { case: "snake", value: {} } }),
    [{ bot: null }],
    "Sam",
  );
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/:any(.*)", component: MatchView }],
  });
  const view = shallowMount(MatchView, { global: { plugins: [router] }, attachTo: document.body });
  await flushPromises();
  return view;
}

async function press(key: string, target: Element = document.body): Promise<void> {
  target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  await flushPromises();
}

describe("MatchView", () => {
  beforeEach(() => {
    screen.keptOn = false;
    playerPauses.length = 0;
    setActivePinia(createPinia());
  });

  it("keeps the screen on while a replay plays", async () => {
    await watchingReplay();
    expect(screen.keptOn).toBe(true);
  });

  it("lets the screen go dark while a replay is paused", async () => {
    const replay = await watchingReplay();
    replay.pause();
    await flushPromises();
    expect(screen.keptOn).toBe(false);
    replay.play();
    await flushPromises();
    expect(screen.keptOn).toBe(true);
  });

  it("pauses a local real-time match with F9 and resumes it at once", async () => {
    const view = await playingLocally();

    await press("F9");
    expect(view.text()).toContain("Paused");
    await press("F9");

    expect(playerPauses).toEqual(["paused", "resumed in 0 ms"]);
    expect(view.text()).not.toContain("Paused");
    view.unmount();
  });

  it("leaves Space to the game (Stack Attack jumps with it)", async () => {
    const view = await playingLocally();

    await press(" ");

    expect(playerPauses).toEqual([]);
    view.unmount();
  });
});
