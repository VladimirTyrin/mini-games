/**
 * localStorage that never throws (private mode, disabled storage, quota) — preferences are a
 * convenience, the app must work without them.
 */
function safeStorage(pick: () => Storage) {
  return {
    get(key: string): string | null {
      try {
        return pick().getItem(key);
      } catch {
        return null;
      }
    },
    set(key: string, value: string): void {
      try {
        pick().setItem(key, value);
      } catch {
        // Not persisted; the in-memory value still applies.
      }
    },
    remove(key: string): void {
      try {
        pick().removeItem(key);
      } catch {
        // Nothing to clean up.
      }
    },
  };
}

export const storage = safeStorage(() => window.localStorage);

/** Per tab: two tabs are two players, but a reload stays the same player. */
export const tabStorage = safeStorage(() => window.sessionStorage);
