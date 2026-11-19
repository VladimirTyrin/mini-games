import { registerPlugin } from "@capacitor/core";

import type { Locale } from "@/i18n";

interface AppLocalePlugin {
  set(options: { languageTag: string }): Promise<void>;
}

/** The app's own plugin (MainActivity): Android words push notifications in this language. */
const nativeAppLocale = registerPlugin<AppLocalePlugin>("AppLocale");

/**
 * Tells Android the app's language: the player's pick, or `null` to follow the phone's. A
 * language only taken from the phone must not be pinned: Android would keep it after the phone
 * changes language, and the WebView, reporting it as the device's, would keep the app in it too.
 * Best effort: an older app without the plugin keeps the phone's.
 */
export async function setNativeAppLocale(locale: Locale | null): Promise<void> {
  if (!__NATIVE_APP__) return;
  try {
    await nativeAppLocale.set({ languageTag: locale ?? "" });
  } catch {
    // An older app shell running a newer web bundle.
  }
}
