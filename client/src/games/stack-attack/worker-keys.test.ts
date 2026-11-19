import { describe, expect, it } from "vitest";

import { workerActionFromKey } from "./worker-keys";

describe("workerActionFromKey", () => {
  it.each([
    ["ArrowUp", "jump"],
    ["w", "jump"],
    ["W", "jump"],
    ["ArrowLeft", "left"],
    ["D", "right"],
    [" ", "jump"],
  ])("%s", (key, expected) => {
    expect(workerActionFromKey(key)).toBe(expected);
  });
});
