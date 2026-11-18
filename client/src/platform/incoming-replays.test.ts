import { describe, expect, it } from "vitest";

import { incomingReplayFile } from "./incoming-replays";

describe("incomingReplayFile", () => {
  it("names the copied file the Android app hands on", () => {
    expect(incomingReplayFile("minigames-replay://open?file=Snake%202026-11-18.mgreplay")).toBe(
      "Snake 2026-11-18.mgreplay",
    );
  });

  it("takes no other link, and no name that leaves the directory", () => {
    expect(incomingReplayFile("https://braintvsminigames.xyz/f/abc")).toBeNull();
    expect(incomingReplayFile("minigames-replay://open?file=..%2Fsecrets")).toBeNull();
    expect(incomingReplayFile("minigames-replay://open")).toBeNull();
  });
});
