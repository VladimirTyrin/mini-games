import { describe, expect, it } from "vitest";

import { normalizeServerUrl } from "./server-url";

describe("normalizeServerUrl", () => {
  it.each([
    ["146.103.106.44", "ws://146.103.106.44/ws"],
    ["146.103.106.44:5000", "ws://146.103.106.44:5000/ws"],
    ["https://braintvsminigames.xyz", "wss://braintvsminigames.xyz/ws"],
    ["http://localhost:5000/", "ws://localhost:5000/ws"],
    ["wss://example.org/custom", "wss://example.org/custom"],
  ])("%s → %s", (input, expected) => {
    expect(normalizeServerUrl(input)).toBe(expected);
  });

  it.each(["", "   ", "ftp://example.org", "http://"])("rejects %j", (input) => {
    expect(normalizeServerUrl(input)).toBeNull();
  });
});
