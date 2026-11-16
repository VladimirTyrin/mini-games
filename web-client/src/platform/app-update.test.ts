import { describe, expect, it } from "vitest";

import { findUpdate, updateInfoUrl } from "./app-update";

describe("updateInfoUrl", () => {
  it("points at the downloads of the game server", () => {
    expect(updateInfoUrl("wss://example.org/ws")?.toString()).toBe(
      "https://example.org/downloads/android.json",
    );
    expect(updateInfoUrl("ws://10.0.0.5:8080/ws")?.toString()).toBe(
      "http://10.0.0.5:8080/downloads/android.json",
    );
  });

  it("refuses addresses that are not WebSocket URLs", () => {
    expect(updateInfoUrl("not a url")).toBeNull();
    expect(updateInfoUrl("https://example.org/")).toBeNull();
  });
});

describe("findUpdate", () => {
  const published = { build: 42, versionName: "2.0.0 (42)", apk: "mini-games.apk" };

  it("offers a newer build with an absolute download link", async () => {
    const update = await findUpdate("wss://example.org/ws", 41, () => Promise.resolve(published));
    expect(update).toEqual({
      versionName: "2.0.0 (42)",
      downloadUrl: "https://example.org/downloads/mini-games.apk",
    });
  });

  it("offers nothing when this build is current or the answer is malformed", async () => {
    expect(await findUpdate("wss://example.org/ws", 42, () => Promise.resolve(published))).toBeNull();
    expect(await findUpdate("wss://example.org/ws", 1, () => Promise.resolve({ build: "x" }))).toBeNull();
  });
});
