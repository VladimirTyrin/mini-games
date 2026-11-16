import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { shallowRef } from "vue";

import type { MatchController } from "@/match/match-controller";

import MatchOverlay from "./MatchOverlay.vue";

vi.mock("@/games", () => ({
  pluginBySettings: (game: string | undefined) => (game === "stackAttack" ? { playsWithSpace: true } : {}),
}));

/** A controller whose match has ended with its result on screen. */
function endedMatch(game: string): MatchController {
  return {
    aborted: shallowRef(false),
    ended: shallowRef(true),
    mySlot: 0,
    pauses: shallowRef([]),
    result: shallowRef(null),
    resultRevealAt: shallowRef(0),
    resumeAt: shallowRef(null),
    source: { kind: "local", header: { settings: { game: { case: game } }, seats: [] } },
  } as unknown as MatchController;
}

function playAgainRequests(game: string, key: string): number {
  const wrapper = mount(MatchOverlay, { props: { controller: endedMatch(game) }, attachTo: document.body });
  window.dispatchEvent(new KeyboardEvent("keydown", { key }));
  const requests = wrapper.emitted("again")?.length ?? 0;
  wrapper.unmount();
  return requests;
}

describe("MatchOverlay", () => {
  it("starts the next match with Enter or Space", () => {
    expect(playAgainRequests("puzzle2048", "Enter")).toBe(1);
    expect(playAgainRequests("puzzle2048", " ")).toBe(1);
  });

  it("does not take Space as Play again in a game that jumps with it", () => {
    expect(playAgainRequests("stackAttack", " ")).toBe(0);
    expect(playAgainRequests("stackAttack", "Enter")).toBe(1);
  });
});
