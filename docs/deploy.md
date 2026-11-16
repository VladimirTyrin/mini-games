# Deploy

`./deploy.sh` builds the server (musl) and the web client and installs both on the host over
SSH (nginx in front, TLS via certbot by default). `./deploy.sh --help` lists the options;
`--apk FILE` also publishes the Android app and its over-the-air web bundle
(see [android.md](android.md), "Updates").

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

1. refuses to start when `android.json` or `web.json` next to the `--apk` file carry another
   build (rebuild the APK from this commit, or pass `--allow-version-mismatch` to publish an APK
   from another commit on purpose);
2. builds the web client and the server with `MG_BUILD_NUMBER` (and `MG_COMMIT` for
   `version.json`), and refuses to upload a `client/dist` of another build
   (a stale `--skip-build`);
3. after the switch, reads `/version`, `/version.json` and, with `--apk`, the two release files
   from the public address (`https://<domain>`, or `http://<host>` in `--mode http`) and fails
   with a list of what differs.

## How clients pick up a deploy

- Browser and installed web app (PWA, including the iOS home screen): the service worker
  precaches the bundle. The app asks the browser to re-check `sw.js` whenever it comes back to the
  foreground and every 30 minutes, since iOS resumes a home-screen app without a navigation.
  A new version is applied (the page reloads) right away, or as soon as the player leaves the
  match on screen (`client/src/platform/web-updates.ts`).
- Android app: see [android.md](android.md), "Updates".
