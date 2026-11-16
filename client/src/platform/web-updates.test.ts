import { afterEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import type { RegisterSWOptions } from "vite-plugin-pwa/types";

import { type RegisterServiceWorker, startWebUpdates, UPDATE_CHECK_INTERVAL_MS } from "./web-updates";

interface FakeServiceWorker {
  readonly register: RegisterServiceWorker;
  readonly applied: boolean[];
  /** The browser found a new version waiting. */
  newVersionWaiting(): void;
  readonly updateChecks: () => number;
}

function fakeServiceWorker(): FakeServiceWorker {
  let options: RegisterSWOptions = {};
  let checks = 0;
  const applied: boolean[] = [];
  const registration = {
    update: () => {
      checks += 1;
      return Promise.resolve();
    },
  } as unknown as ServiceWorkerRegistration;
  return {
    register(given) {
      options = given;
      given.onRegisteredSW?.("sw.js", registration);
      return (reloadPage) => {
        applied.push(reloadPage ?? false);
        return Promise.resolve();
      };
    },
    applied,
    newVersionWaiting: () => options.onNeedRefresh?.(),
    updateChecks: () => checks,
  };
}

function setVisibility(state: DocumentVisibilityState): void {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
  document.dispatchEvent(new Event("visibilitychange"));
}

afterEach(() => {
  setVisibility("visible");
});

describe("web updates", () => {
  it("applies a new version at once when no match is open", () => {
    const worker = fakeServiceWorker();
    startWebUpdates(worker.register, () => false);

    worker.newVersionWaiting();

    expect(worker.applied).toEqual([true]);
  });

  it("holds a new version during a match and applies it once the player leaves", async () => {
    const worker = fakeServiceWorker();
    const inMatch = ref(true);
    startWebUpdates(worker.register, () => inMatch.value);

    worker.newVersionWaiting();
    expect(worker.applied).toEqual([]);

    inMatch.value = false;
    await nextTick();
    expect(worker.applied).toEqual([true]);
  });

  it("checks for a new version whenever the app comes back to the foreground", () => {
    const worker = fakeServiceWorker();
    startWebUpdates(worker.register, () => false);
    const before = worker.updateChecks();

    setVisibility("hidden");
    expect(worker.updateChecks()).toBe(before);
    setVisibility("visible");
    expect(worker.updateChecks()).toBe(before + 1);
  });

  it("checks every half hour while the app is open", () => {
    vi.useFakeTimers();
    try {
      const worker = fakeServiceWorker();
      startWebUpdates(worker.register, () => false);
      const before = worker.updateChecks();

      vi.advanceTimersByTime(UPDATE_CHECK_INTERVAL_MS);

      expect(worker.updateChecks()).toBe(before + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
