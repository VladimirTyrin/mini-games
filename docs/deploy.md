# Deploy

`./deploy.sh` builds everything from the checked-out commit and installs it on the host over
SSH (nginx in front, TLS via certbot by default): the Android app and its over-the-air web bundle
(see [android.md](android.md), "Updates"), the web client and the server (musl).
`./deploy.sh --help` lists the options; `--no-apk` leaves the app out, `--apk FILE` publishes a
prebuilt one.

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

## How clients pick up a deploy

- Browser and installed web app (PWA, including the iOS home screen): the service worker
  precaches the bundle. The app asks the browser to re-check `sw.js` whenever it comes back to the
  foreground and every 30 minutes, since iOS resumes a home-screen app without a navigation.
  A new version is applied (the page reloads) right away, or as soon as the player leaves the
  match on screen or the online lobby (`client/src/platform/web-updates.ts`). A match whose
  history the old tab can no longer replay (the rules changed) says so and lets the update in.
- Android app: see [android.md](android.md), "Updates".
