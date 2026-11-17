import { afterEach, describe, expect, it, vi } from "vitest";

import { ScreenWakeLock } from "./keep-awake";

/** A browser wake lock that, like the real one, drops its locks when the page is hidden. */
function fakeWakeLock(): { wakeLock: WakeLock; held: () => number; hide: () => void } {
  const locks = new Set<WakeLockSentinel>();
  const wakeLock = {
    request: () => {
      const sentinel = {
        released: false,
        release() {
          sentinel.released = true;
          locks.delete(sentinel as unknown as WakeLockSentinel);
          return Promise.resolve();
        },
      };
      locks.add(sentinel as unknown as WakeLockSentinel);
      return Promise.resolve(sentinel as unknown as WakeLockSentinel);
    },
  } as unknown as WakeLock;
  return {
    wakeLock,
    held: () => locks.size,
    hide: () => {
      for (const lock of [...locks]) void lock.release();
    },
  };
}

function setVisibility(state: DocumentVisibilityState): void {
  vi.spyOn(document, "visibilityState", "get").mockReturnValue(state);
  document.dispatchEvent(new Event("visibilitychange"));
}

/** Lets the lock's chain of promises, started by a visibility event, run to its end. */
async function settle(): Promise<void> {
  for (let turn = 0; turn < 5; turn += 1) await Promise.resolve();
}

describe("ScreenWakeLock", () => {
  afterEach(() => vi.restoreAllMocks());

  it("holds the screen on while wanted and lets it go after", async () => {
    const browser = fakeWakeLock();
    const lock = new ScreenWakeLock(browser.wakeLock, document);

    await lock.set(true);
    expect(browser.held()).toBe(1);

    await lock.set(false);
    expect(browser.held()).toBe(0);
  });

  it("takes the lock again when the page comes back", async () => {
    const browser = fakeWakeLock();
    const lock = new ScreenWakeLock(browser.wakeLock, document);
    await lock.set(true);

    setVisibility("hidden");
    browser.hide();
    await settle();
    expect(browser.held()).toBe(0);

    setVisibility("visible");
    await settle();
    expect(browser.held()).toBe(1);
  });

  it("takes one lock however often it is asked", async () => {
    const browser = fakeWakeLock();
    const lock = new ScreenWakeLock(browser.wakeLock, document);

    await Promise.all([lock.set(true), lock.set(true), lock.set(true)]);

    expect(browser.held()).toBe(1);
  });

  it("gives back a lock that arrives after it was let go", async () => {
    const browser = fakeWakeLock();
    const lock = new ScreenWakeLock(browser.wakeLock, document);

    await Promise.all([lock.set(true), lock.set(false)]);

    expect(browser.held()).toBe(0);
  });

  it("does nothing where the browser has no wake lock", async () => {
    const lock = new ScreenWakeLock(undefined, document);

    await lock.set(true);

    expect(lock.isHeld).toBe(false);
  });
});
