// Push handlers of the service worker (vite.config.ts adds this file to the generated worker,
// after push-texts.js, which defines self.PUSH_TEXTS).
// Payloads come from the server's PushNotification: { title, body, tag, data } in English, and
// { titleKey, titleArgs, bodyKey, bodyArgs }, worded here in the app's language.

/** The app's language, kept for the worker in IndexedDB (src/platform/worker-locale.ts). */
function readLocale() {
  return new Promise((resolve) => {
    try {
      const opening = indexedDB.open("mg-worker", 1);
      opening.onupgradeneeded = () => opening.result.createObjectStore("prefs");
      opening.onerror = () => resolve("en");
      opening.onsuccess = () => {
        const database = opening.result;
        const answer = (locale) => {
          database.close();
          resolve(locale);
        };
        try {
          const reading = database.transaction("prefs").objectStore("prefs").get("locale");
          reading.onsuccess = () => answer(reading.result || "en");
          reading.onerror = () => answer("en");
        } catch {
          answer("en");
        }
      };
    } catch {
      resolve("en");
    }
  });
}

/** The text of `key` with `%1$s`-style arguments, or `fallback` when the key is unknown. */
function word(texts, key, args, fallback) {
  const template = key && texts[key];
  if (!template) return fallback;
  return template.replace(/%(\d+)\$s/g, (_, position) => String((args || [])[Number(position) - 1] ?? ""));
}

self.addEventListener("push", (event) => {
  const receivedAt = Date.now();
  let message = {};
  try {
    message = event.data ? event.data.json() : {};
  } catch {
    message = {};
  }
  const data = { ...(message.data || {}), receivedAt: String(receivedAt) };
  // Safari cancels the subscription of a site whose push shows nothing, so every push shows a
  // notification.
  event.waitUntil(
    (async () => {
      const locale = await readLocale();
      const all = self.PUSH_TEXTS || {};
      const texts = all[locale] || all.en || {};
      const title = word(texts, message.titleKey, message.titleArgs, message.title || "Mini Games");
      let body = word(texts, message.bodyKey, message.bodyArgs, message.body || "");
      const sentAt = Number(data.sentAt);
      if (data.kind === "test" && sentAt > 0) {
        // "1.5" / "1,5": the number as the language writes it.
        const seconds = new Intl.NumberFormat(locale, {
          minimumFractionDigits: 1,
          maximumFractionDigits: 1,
        }).format(Math.max(0, (receivedAt - sentAt) / 1000));
        body = word(texts, "push_test_delivered", [seconds], body);
      }
      await self.registration.showNotification(title, { body, tag: message.tag, data, icon: "icon-192.png" });
    })(),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const data = { ...(event.notification.data || {}), tappedAt: String(Date.now()) };
  event.waitUntil(
    (async () => {
      const windows = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
      const open = windows[0];
      if (open) {
        await open.focus();
        open.postMessage({ type: "notification-opened", data });
        return;
      }
      const url = new URL(self.registration.scope);
      url.searchParams.set("notification", JSON.stringify(data));
      await self.clients.openWindow(url.href);
    })(),
  );
});
