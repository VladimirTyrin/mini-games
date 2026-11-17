import { registerPlugin } from "@capacitor/core";

interface KeepAwakePlugin {
  set(options: { keepScreenOn: boolean }): Promise<void>;
}

/** The app's own plugin (MainActivity): Android's WebView has no Screen Wake Lock API. */
const nativeKeepAwake = registerPlugin<KeepAwakePlugin>("KeepAwake");

/**
 * Holds the browser's screen wake lock while wanted. The browser drops the lock whenever the page
 * is hidden, so it is taken again each time the page comes back.
 */
export class ScreenWakeLock {
  private wanted = false;
  private sentinel: WakeLockSentinel | null = null;
  private isRequesting = false;

  constructor(
    private readonly wakeLock: WakeLock | undefined,
    private readonly page: Document,
  ) {
    page.addEventListener("visibilitychange", () => void this.sync());
  }

  set(keepScreenOn: boolean): Promise<void> {
    this.wanted = keepScreenOn;
    return this.sync();
  }

  get isHeld(): boolean {
    return this.sentinel !== null && !this.sentinel.released;
  }

  private async sync(): Promise<void> {
    if (!this.wakeLock) return;
    if (!this.wanted) {
      const held = this.sentinel;
      this.sentinel = null;
      await held?.release();
      return;
    }
    if (this.isRequesting || this.isHeld || this.page.visibilityState !== "visible") return;
    this.isRequesting = true;
    let sentinel: WakeLockSentinel;
    try {
      sentinel = await this.wakeLock.request("screen");
    } catch {
      // Refused (battery saver, policy): the screen just sleeps as usual.
      return;
    } finally {
      this.isRequesting = false;
    }
    // Let go while the request was in flight: give the late lock back at once.
    if (this.wanted) this.sentinel = sentinel;
    else await sentinel.release();
  }
}

let browserLock: ScreenWakeLock | null = null;

/** Keeps the screen on, or lets it sleep again. Best effort: nothing breaks where it is refused. */
export async function setKeepScreenOn(keepScreenOn: boolean): Promise<void> {
  if (__NATIVE_APP__) {
    try {
      await nativeKeepAwake.set({ keepScreenOn });
    } catch {
      // An older app without the plugin, running a newer web bundle.
    }
    return;
  }
  browserLock ??= new ScreenWakeLock(navigator.wakeLock, document);
  await browserLock.set(keepScreenOn);
}
