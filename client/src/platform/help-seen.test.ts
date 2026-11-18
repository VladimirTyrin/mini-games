import { beforeEach, describe, expect, it } from "vitest";

import { hasSeenHelp, markHelpSeen } from "./help-seen";

describe("help seen", () => {
  beforeEach(() => localStorage.clear());

  it("remembers per game that its help was shown", () => {
    markHelpSeen("numbers_match");

    expect(hasSeenHelp("numbers_match")).toBe(true);
    expect(hasSeenHelp("snake")).toBe(false);
  });

  it("treats a mangled record as nothing seen", () => {
    localStorage.setItem("mini-games.help-seen", "{not json");

    expect(hasSeenHelp("numbers_match")).toBe(false);
  });
});
