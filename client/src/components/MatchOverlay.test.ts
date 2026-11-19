import { create } from "@bufbuild/protobuf";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { shallowRef } from "vue";

import type { MatchController } from "@/match/match-controller";
import { IdentityKind, IdentitySchema } from "@/proto/minigames/v2/identity_pb";
import { LobbyStateSchema, MatchSeriesSchema } from "@/proto/minigames/v2/lobby_pb";
import { MatchResultSchema, OutcomeKind } from "@/proto/minigames/v2/match_pb";
import { useConnectionStore } from "@/stores/connection";
import { useLobbyStore } from "@/stores/lobby";

import MatchOverlay from "./MatchOverlay.vue";

vi.mock("@/games", () => ({
  pluginBySettings: (game: string | undefined) => ({ id: game }),
}));
vi.mock("@/wasm/core", () => ({
  gameCatalog: () => [
    { id: "tictactoe", scores: "winFlag" },
    { id: "snake", scores: "points" },
  ],
}));

/** Two players, the second won this match: the rows of the result, cells split by "|". */
function resultRows(game: string, scores: bigint[]): string[] {
  const controller = endedMatch(game, {
    result: shallowRef(create(MatchResultSchema, { kind: OutcomeKind.WINNER, winnerSlots: [1], scores })),
    source: {
      kind: "local",
      header: {
        settings: { game: { case: game } },
        seats: [
          { slot: 0, player: { id: "ann", displayName: "Ann" } },
          { slot: 1, player: { id: "bob", displayName: "Bob" } },
        ],
      },
    },
  });
  const series = create(MatchSeriesSchema, { wins: [2, 3], draws: 1 });
  const wrapper = mount(MatchOverlay, { props: { controller, series } });
  const cell = (element: Element) => (element.textContent ?? "").trim().replace(/\s+/g, " ");
  const rows = wrapper.findAll("li").map((row) => {
    const cells = Array.from(row.element.children).map(cell).filter(Boolean);
    return cells.length > 0 ? cells.join(" | ") : cell(row.element);
  });
  wrapper.unmount();
  return rows;
}

/** A controller whose match has ended with its result on screen. */
function endedMatch(game: string, changes: Partial<Record<string, unknown>> = {}): MatchController {
  return {
    aborted: shallowRef(false),
    droppedSlots: shallowRef(new Set()),
    ended: shallowRef(true),
    mySlot: 0,
    pauses: shallowRef([]),
    result: shallowRef(null),
    resultRevealAt: shallowRef(0),
    resumeAt: shallowRef(null),
    source: { kind: "local", header: { settings: { game: { case: game } }, seats: [] } },
    ...changes,
  } as unknown as MatchController;
}

function playAgainRequests(game: string, key: string): number {
  const wrapper = mount(MatchOverlay, { props: { controller: endedMatch(game) }, attachTo: document.body });
  window.dispatchEvent(new KeyboardEvent("keydown", { key }));
  const requests = wrapper.emitted("again")?.length ?? 0;
  wrapper.unmount();
  return requests;
}

const buttons = (controller: MatchController) => {
  const wrapper = mount(MatchOverlay, { props: { controller } });
  const texts = wrapper.findAll("button").map((button) => button.text());
  wrapper.unmount();
  return texts;
};

describe("MatchOverlay", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("offers Play again to someone watching bots on this device", () => {
    expect(buttons(endedMatch("snake", { mySlot: null })).some((text) => text.startsWith("Play again"))).toBe(
      true,
    );
  });

  it("offers Play again to the lobby's host watching a table of bots", () => {
    const host = create(IdentitySchema, { kind: IdentityKind.GUEST, id: "host", displayName: "Host" });
    useConnectionStore().identity = host;
    useLobbyStore().current = create(LobbyStateSchema, { lobbyId: "lobby", host });
    const botsOnly = {
      kind: "online",
      header: { settings: { game: { case: "snake" } }, seats: [{ botKind: {} }, { botKind: {} }] },
    };

    expect(
      buttons(endedMatch("snake", { mySlot: null, source: botsOnly })).some((text) =>
        text.startsWith("Play again"),
      ),
    ).toBe(true);
  });

  it("shows the wins of the series instead of a 1 for the winner", () => {
    expect(resultRows("tictactoe", [0n, 1n])).toEqual(["Wins", "Ann (you) | 2", "Bob | 3", "Draws: 1"]);
  });

  it("shows the wins of the series beside points of the game's own", () => {
    expect(resultRows("snake", [7n, 12n])).toEqual([
      "Score | Wins",
      "Ann (you) | 7 | 2",
      "Bob | 12 | 3",
      "Draws: 1",
    ]);
  });

  it("starts the next match with Enter", () => {
    expect(playAgainRequests("puzzle2048", "Enter")).toBe(1);
    expect(playAgainRequests("stackAttack", "Enter")).toBe(1);
  });

  it("does not start the next match with Space, the pause key, pressed as the game ends", () => {
    expect(playAgainRequests("stackAttack", " ")).toBe(0);
    expect(playAgainRequests("puzzle2048", " ")).toBe(0);
  });

  it("offers Play again neither to an observer of people online nor after an aborted match", () => {
    const peopleOnline = {
      kind: "online",
      header: { settings: { game: { case: "tictactoe" } }, seats: [{}, {}] },
    };
    expect(buttons(endedMatch("tictactoe")).some((text) => text.startsWith("Play again"))).toBe(true);
    expect(
      buttons(endedMatch("tictactoe", { mySlot: null, source: peopleOnline })).some((text) =>
        text.startsWith("Play again"),
      ),
    ).toBe(false);
    expect(
      buttons(endedMatch("tictactoe", { aborted: shallowRef(true) })).some((text) =>
        text.startsWith("Play again"),
      ),
    ).toBe(false);
  });
});
