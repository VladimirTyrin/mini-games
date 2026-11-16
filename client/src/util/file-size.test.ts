import { describe, expect, it } from "vitest";

import { formatFileSize } from "./file-size";

describe("formatFileSize", () => {
  it.each([
    [0, "0 B"],
    [850, "850 B"],
    [1024, "1.0 KB"],
    [12_595, "12.3 KB"],
    [1_258_291, "1.2 MB"],
  ])("%d bytes → %s", (bytes, expected) => {
    expect(formatFileSize(bytes)).toBe(expected);
  });
});
