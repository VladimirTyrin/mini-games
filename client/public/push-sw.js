// Push handlers of the service worker (vite.config.ts adds this file to the generated worker).
// Payloads come from the server's PushNotification: { title, body, tag, data }.

self.addEventListener("push", (event) => {
  const receivedAt = Date.now();
  let message = {};
  try {
    message = event.data ? event.data.json() : {};
  } catch {
    message = {};
  }
  const data = { ...(message.data || {}), receivedAt: String(receivedAt) };
  let body = message.body || "";
  const sentAt = Number(data.sentAt);
  if (data.kind === "test" && sentAt > 0) {
    const seconds = Math.max(0, (receivedAt - sentAt) / 1000).toFixed(1);
    body = `Test notification: delivered in ${seconds} s. Tap to open the app.`;
  }
  // Safari cancels the subscription of a site whose push shows nothing, so every push shows a
  // notification.
  event.waitUntil(
    self.registration.showNotification(message.title || "Mini Games", {
      body,
      tag: message.tag,
      data,
      icon: "icon-192.png",
    }),
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
