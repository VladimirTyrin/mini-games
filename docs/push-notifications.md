# Push notifications

The server sends push notifications to two kinds of device:
- the Android app, through **FCM**;
- browsers and installed web apps, through **Web Push**. On iPhone this needs iOS 16.4+ and the
  app added to the Home Screen; Safari tabs get no push.

Every device of a Google account registers its address (`RegisterPushTarget`, see
[protocol.md](protocol.md), "Friends and invitations"), and a notification goes to all of them:

| What | Tap opens |
|---|---|
| A friend invites you into a lobby (lives 90 s) | the app, which accepts and joins |
| A friend request, a new friend | the Friends screen |
| The test in Settings ("Send a test notification", 20 s after the tap) | Settings, with how long each step took |

An invitation answered on one device is replaced on the friend's other devices ("you joined on
another device"). This happens only when the server knows which device answered, from the push
address it registered after signing in; otherwise the others keep the stale invitation, which is
better than a "joined on another device" on the device that just joined. When an invitation expires or is cancelled, no second notification goes out:
it would land on every device that ignored the first one. Tapping the stale one says the
invitation is closed. While the app is open, invitations also show as a banner in the app (an
open browser tab shows both).

Nobody gets the same news from the same sender over and over:
- one notification a day per sender for a friend request or a new friend;
- one every 10 s per friend for invitations.

Sending goes through a small queue: at most 2 sends run at once and 256 wait. Past that,
notifications are dropped, so a slow push service cannot fill the blocking pool that the database
and bots share.

A device stops getting an account's notifications when the account signs out on it. The app asks
for notification permission once, the first time the player adds a friend, answers a request,
shares or opens a friend link, or invites someone. It asks from that tap, since iOS lets nothing
else show the prompt. After that, the Friends screen offers it.

On iPhone, links from other apps always open in Safari, never in the Home Screen app. That
matters only for friend links: the Friends screen takes a pasted link, and a request sent with
"+ Friend" in a lobby arrives as a notification, which opens the app itself.

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
