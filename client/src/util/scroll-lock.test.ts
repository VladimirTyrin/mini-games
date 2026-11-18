import { afterEach, describe, expect, it } from "vitest";

import { lockPageScroll } from "./scroll-lock";

describe("lockPageScroll", () => {
  afterEach(() => {
    document.documentElement.style.overflow = "";
  });

  it("keeps the page still until the last dialog over it closes", () => {
    const releaseHelp = lockPageScroll();
    const releaseConfirm = lockPageScroll();
    expect(document.documentElement.style.overflow).toBe("hidden");

    releaseConfirm();
    releaseConfirm();
    expect(document.documentElement.style.overflow).toBe("hidden");
    releaseHelp();

    expect(document.documentElement.style.overflow).toBe("");
  });
});
