import type { Locale } from "@/i18n";

/**
 * Keeps the app's language where the service worker can read it (it has no localStorage), so
 * a Web Push is worded in it (public/push-sw.js reads the same database).
 */
export function keepLocaleForWorker(locale: Locale): void {
  if (typeof indexedDB === "undefined") return;
  try {
    const opening = indexedDB.open("mg-worker", 1);
    opening.onupgradeneeded = () => opening.result.createObjectStore("prefs");
    opening.onsuccess = () => {
      const database = opening.result;
      try {
        const writing = database.transaction("prefs", "readwrite");
        writing.objectStore("prefs").put(locale, "locale");
        writing.oncomplete = writing.onerror = writing.onabort = () => database.close();
      } catch {
        // A push is then worded in English; nothing else depends on it.
        database.close();
      }
    };
  } catch {
    // As above.
  }
}
