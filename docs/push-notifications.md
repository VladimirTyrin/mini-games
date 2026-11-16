# Push notifications

Groundwork for "call a friend to a game now". It covers two kinds of device:

- the Android app, through **FCM**;
- browsers and installed web apps, through **Web Push**. On iPhone this means iOS 16.4+ with the
  app added to the Home Screen; Safari tabs get no push.

So far the only notification is the test in Settings: "Send a test notification" comes 20 s
after the tap, which leaves time to close the app or lock the phone. Settings then shows how
long each step took.

## Flow

1. The device asks for permission and gets its push address:
   - the app gets an FCM registration token (`@capacitor/push-notifications`);
   - a browser gets a `PushSubscription` for the server's VAPID public key
     (`ServerInfo.web_push_public_key`).
   - Code: `client/src/platform/push-notifications.ts`.
2. The device sends `RequestTestPush { target, delay_seconds }` (see [protocol.md](protocol.md)).
3. After the delay the server sends the message (`crates/server/src/push/`):
   - FCM: the HTTP v1 API, signed in as the Firebase service account (OAuth JWT bearer). The
     message uses priority `HIGH` and channel `invites`; the app creates that channel with high
     importance, so the notification pops up.
   - Web Push (RFC 8030): the payload is encrypted for the subscription (RFC 8291,
     `aes128gcm`) and signed with the VAPID key (RFC 8292).
4. Each payload carries `title`, `body`, `tag` and `data`:
   - `data` holds `kind` and `sentAt`, the server clock in unix ms.
   - The service worker (`client/public/push-sw.js`) adds `receivedAt` and must show a
     notification for every push. Safari cancels the subscription of a site whose push shows
     nothing.
   - A tap adds `tappedAt` and focuses the open window, or opens the app with the data in the
     `?notification=` parameter.

The server sends Web Push only to the browsers' push services (`WebPushEndpoint`):
- Google;
- Mozilla;
- Apple;
- Windows.

Without this list, a client could make the server send requests to any address.

## Credentials

| What | Where | If lost |
|---|---|---|
| `client/android/app/google-services.json` | in git. Firebase calls it non-secret: it identifies the project, and the app needs it to get FCM tokens | download it again from the Firebase console (project `mini-games-2630d`) |
| Firebase service account key | `.keys/*-firebase-adminsdk-*.json` locally; `deploy.sh` writes it to the host as `MG_FCM_SERVICE_ACCOUNT` (base64) | generate a new one in the Firebase console (Project settings, Service accounts) and deploy |
| VAPID private key | only on the host: `MG_VAPID_PRIVATE_KEY`, made once by `deploy.sh` | a new key is made; browsers subscribe again with it the next time they register |

`MG_VAPID_SUBJECT` is `https://<domain>`. Each kind of push is off while its setting is empty,
and the server then answers `ERROR_CODE_PUSH_UNAVAILABLE`.
