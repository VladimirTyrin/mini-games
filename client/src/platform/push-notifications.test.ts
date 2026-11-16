import { afterEach, describe, expect, it, vi } from "vitest";

import { onNotification } from "./push-notifications";

describe("onNotification", () => {
  afterEach(() => history.replaceState(null, "", "/"));

  it("hands over the notification that started the app and clears it from the address", async () => {
    const data = { kind: "test", sentAt: "1000", receivedAt: "1500", tappedAt: "4000" };
    history.replaceState(null, "", `/?notification=${encodeURIComponent(JSON.stringify(data))}#/`);
    const handler = vi.fn();

    await onNotification(handler);

    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "test", sentAt: 1000, receivedAt: 1500, tappedAt: 4000 }),
    );
    expect(location.search).toBe("");
    expect(location.hash).toBe("#/");
  });

  it("ignores an address parameter that is not a notification", async () => {
    history.replaceState(null, "", "/?notification=nonsense");
    const handler = vi.fn();

    await onNotification(handler);

    expect(handler).not.toHaveBeenCalled();
  });
});
