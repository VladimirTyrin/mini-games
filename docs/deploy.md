# Deploy

`./deploy.sh` builds everything from the checked-out commit and installs it on the host over
SSH (nginx in front, TLS via certbot by default): the Android app and its over-the-air web bundle
(see [android.md](android.md), "Updates"), the web client and the server (musl).
`./deploy.sh --help` lists the options; `--no-apk` leaves the app out, `--apk FILE` publishes a
prebuilt one.

## When to deploy

Right after the push, without waiting for CI: run the checks of AGENTS.md ("Commands", and the
client's lint, typecheck and tests) before pushing, and the deploy's own build stops on a failing
web build or size budget. CI runs alongside; a red run is fixed with the next commit.

## Web only

`./deploy.sh --web-only` releases the web client alone, in well under a minute: the site and the
app's over-the-air bundle. The server is neither built nor restarted, so running matches go on.
It builds `npm run android:bundle` (the bundle without Gradle) and the site, uploads both, swaps
the site's directory on the host and checks `/version.json` and `/downloads/web.json`.

The web client plays by the server's rules (the WASM core, the protocol), so the script refuses
and asks for the whole deploy when:
- anything the server runs changed since the build the host serves: `crates/`, `proto/`, the
  Cargo files, `deploy/` or `deploy.sh`, committed or not. The host's build comes from `/version`,
  its commit from the build number (the commit count, on the first-parent line of `HEAD`);
- the app's native shell changed (`nativeFingerprint` of the new bundle differs from the
  published `web.json`): installed apps take a bundle only into the shell it names, so the
  change needs a new APK.

After a web-only release the server reports an older build than `/version.json`; the next whole
deploy brings them together again.

The toolchain paths of the machine that deploys (JDK and Android SDK for the app, zig for the
musl server) go to `deploy.local.env` next to the script, which it sources; it is not in git,
`deploy.local.env.example` shows the variables.

## One build everywhere

Every deploy puts exactly one build on the host. The build number is the commit count of
`HEAD` (`git rev-list --count HEAD`), the same number the Android build uses:

| Where | Shows the build as |
|---|---|
| server: `GET /version`, `ServerInfo.server_version` | `2.0.0+512` (`MG_BUILD_NUMBER` at compile time; a build without it says `2.0.0`) |
| web bundle: `GET /version.json` | `{"build":512,"commit":"3a3cf87"}`, written by the Vite build |
| web Settings screen | `Version 2.0.0 (512)` |
| `/downloads/android.json`, `/downloads/web.json` | `"build": 512` (written by `npm run android:apk`) |

`deploy.sh`

1. builds the APK first (`npm run android:apk`: it writes the app's web bundle to
   `client/dist`, which the web build then replaces), then the WASM core and protocol code, the
   web client and the server with `MG_BUILD_NUMBER` (and `MG_COMMIT` for `version.json`);
2. refuses to upload when `android.json` or `web.json` next to the APK carry another build (a
   prebuilt `--apk` from another commit; `--allow-version-mismatch` publishes one on purpose) or
   `client/dist` is of another build (a stale `--skip-build`);
3. after restarting the server, fails unless nginx answers `/healthz` and `/` with 200 on the
   site itself (https in tls mode: a redirect does not count);
4. after the switch, reads `/version`, `/version.json` and, with `--apk`, the two release files
   from the public address (`https://<domain>`, or `http://<host>` in `--mode http`) and fails
   with a list of what differs.

## Restarts and running matches

Lobbies and matches live only in the server's memory, so a restart ends them. Before restarting,
`deploy.sh` asks the server to drain: `POST /admin/drain` on `127.0.0.1:5000`. The endpoint takes
requests from the host only; nginx does not forward it.

While draining:
- the server refuses new matches with `ERROR_CODE_SERVER_UPDATING`;
- every player gets `ServerUpdating`, and the app pauses Start (an offline solo start still
  works);
- matches already running play on.

The endpoint answers how many matches are still running with someone connected; a match every
player left is not waited for. `deploy.sh` asks every 5 s and restarts once the answer is 0, or
after `--drain-seconds` (180 by default). `--no-drain` restarts at once. New matches are refused
for that long, so the wait stays short; a long turn-based game is cut off with a message.

A match or lobby that a restart still cuts off says so in the app ("The server restarted for an
update…") instead of just closing.

## Database

Friends, friend links and push addresses live in SQLite at
`/var/lib/mini-games-server/mini-games.db`. systemd keeps that directory across deploys
(`StateDirectory`), and the server applies new migrations when it starts. A migration that fails
keeps the service from starting, and `deploy.sh` then fails with the journal. There are no
backups yet.

## Secrets on the host

The server reads `/etc/mini-games-server.env`.

`deploy.sh` creates `MG_SESSION_SECRET` and `MG_VAPID_PRIVATE_KEY` there once and keeps them:
- replacing the session secret signs everyone out;
- replacing the VAPID key makes browsers subscribe to push again.

It also writes `MG_GOOGLE_CLIENT_ID` and `MG_VAPID_SUBJECT` on every deploy.

`MG_FCM_SERVICE_ACCOUNT` is copied from the Firebase key in `.keys/` when that key is there.
Without it, the host keeps its own. See [push-notifications.md](push-notifications.md).

## How clients pick up a deploy

- Browser and installed web app (PWA, including the iOS home screen): the service worker
  precaches the bundle. The app asks the browser to re-check `sw.js` whenever it comes back to the
  foreground and every 30 minutes, since iOS resumes a home-screen app without a navigation.
  A new version is applied (the page reloads) right away, or as soon as the player leaves the
  match on screen or the online lobby (`client/src/platform/web-updates.ts`). A match whose
  history the old tab can no longer replay (the rules changed) says so and lets the update in.
- Android app: see [android.md](android.md), "Updates".
