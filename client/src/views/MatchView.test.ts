import { flushPromises, shallowMount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

import type { ReplayMatchSource } from "@/match/replay-match-source";
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
vi.mock("@/match/match-controller", async () => {
  const { shallowRef } = await import("vue");
  return {
    MatchController: class {
      readonly incompatible = shallowRef(false);
      readonly ended = shallowRef(false);
      readonly current = shallowRef(null);
      readonly gameTickMs = 0;
      constructor(readonly source: unknown) {}
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

describe("MatchView", () => {
  beforeEach(() => {
    screen.keptOn = false;
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
});
