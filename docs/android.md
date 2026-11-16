# Android app

The app is the web client inside Capacitor 8 (`client/android`). Games, bots and replays run
on the same WASM core as in the browser; the app adds the system back button, native Google
sign-in and an editable server address (Settings).

## Build locally

Needs JDK 21 and the Android SDK (platform 36, build-tools 36) with `JAVA_HOME` and
`ANDROID_HOME` set, plus the usual web toolchain (see `AGENTS.md`).

```bash
cd client
npm run android:apk          # signed release APK
npm run android:apk:debug    # debug APK
npm run android:sync         # web bundle + cap sync only (then open android/ in Android Studio)
```

The APK lands in `android/app/build/outputs/apk/<variant>/`; a release build is also copied to
`android/dist/mini-games.apk` with `android/dist/android.json` (build number = commit count,
`versionCode = major * 1 000 000 + build`, so every later build installs over an earlier one). `build/android.mjs` builds the web
bundle with `VITE_PLATFORM=android`: only that flavour contains the Capacitor plugins, the web
build drops them at build time. The default server is `wss://braintvsminigames.xyz/ws`; override it with
`MG_ANDROID_SERVER_URL`. Users can change it in Settings (plain `ws://` to an IP is allowed;
nginx serves `/ws` on port 80 too).

## Replays

"Save replay" at the end of a match writes the file into the app's own folder
`Android/data/xyz.braintvsminigames.app/files/replays/` (Capacitor `Directory.External`: no
storage permission, reachable over USB, removed with the app). The name says what it is:
`<game> <local date> <time> <match id start>.mgreplay`, e.g.
`Snake 2026-11-16 14-05-33 3f9a1c.mgreplay`; saving the same match again overwrites it. The
browser build downloads the same file name instead.

Home → "Saved replays" (`/replays`, app only) lists the folder newest first, from the directory
listing alone (game and time from the file name, else the file's modification time), so files
copied in or deleted outside the app show up correctly. Tap to watch; Share opens the system
share sheet; Delete asks first.

## Signing

Release and debug keystores live in the gitignored `.keys/` folder at the repository root, each
with a `<variant>-signing.properties` (`storeFile`, `storePassword`, `keyAlias`,
`keyPassword`). Back them up: an app update must be signed with the same release key.

CI reads the release key from environment variables instead:
`MG_ANDROID_RELEASE_STORE_FILE`, `..._STORE_PASSWORD`, `..._KEY_ALIAS`, `..._KEY_PASSWORD`.

## Google sign-in

The app asks Android's Credential Manager for an ID token whose audience is the **Web** OAuth
client ID (the one the server sends in `ServerInfo`), so the server verifies it like a web
sign-in. Google lets the app request it only if an **Android** OAuth client in the same Google
Cloud project lists package `xyz.braintvsminigames.app` and the SHA-1 of the signing key — one
client per key (release and debug). Print a key's SHA-1 with
`keytool -list -v -keystore .keys/mini-games-release.jks`.

## Push notifications

FCM comes through `@capacitor/push-notifications`.
- `android/app/google-services.json` (Firebase project `mini-games-2630d`, in git) turns on the
  Google services Gradle plugin.
- The app creates the high-importance channel `invites` before registering.
- See [push-notifications.md](push-notifications.md).

## Updates

The app has two parts: the native shell (the `android/` project, the Capacitor plugins and
their config) and the web bundle inside it (JS, CSS, WASM). Most releases change only the web
bundle, and the app picks it up over the air (OTA) without a new APK and without asking;
it asks the player to install an APK only when the shell itself changed. Everything stays
offline-capable: the app always runs a complete bundle stored on the device.

**Shell fingerprint.** `build/native-fingerprint.mjs` hashes everything native: the files of
`android/` (without build outputs, `.gradle`, the copied web bundle and the files `cap sync`
regenerates), `capacitor.config.ts` and the exact versions of the `@capacitor/*` and `@capgo/*`
packages in `package-lock.json`. `build/android.mjs` bakes it into the Android web bundle as
`__NATIVE_FINGERPRINT__`; the web build has `""` there.

**What a release build publishes.** `npm run android:apk` writes to `android/dist/`:

| File | Content |
|---|---|
| `mini-games.apk` | the app |
| `android.json` | `{ build, versionName, apk }` |
| `web-<build>.zip` | the Android web bundle (`dist/`, `index.html` at the zip root) |
| `web.json` | `{ build, versionName, nativeFingerprint, zip, sha256 }` |

`./deploy.sh` builds them and uploads all four to `/downloads/`:
the APK before `android.json`, the zip before `web.json`, so a published file never names one
that is not there yet. The server keeps the 3 newest `web-*.zip`. The APK files must be of the
build being deployed (see [deploy.md](deploy.md)).

**The check.** On start, after the server address changes and on Settings → "Check for
updates", the app reads `web.json` and `android.json` from its server
(`wss://host/ws` → `https://host/downloads/…`) and compares them with the running bundle's
`__APP_BUILD__` and `__NATIVE_FINGERPRINT__`:

| Published | App does |
|---|---|
| `web.json` build newer, same fingerprint, served over https | downloads the zip silently (the plugin checks its SHA-256) and switches to it at once outside a match and an online lobby, otherwise when the app next goes to the background |
| newer build with another fingerprint, or no `web.json` (older server), or a plain-http server | offers "Update available → Download" for the APK, as long as `android.json` is newer |
| nothing newer | nothing |
| a newer build that already failed to start on this device | nothing (it is not retried) |

The OTA part uses `@capgo/capacitor-updater` in manual mode: no Capgo cloud, no update checks of
its own, statistics off (`capacitor.config.ts`). The switch never happens during a match: while
a match is being played the switch is held until the player leaves it (a match this build cannot
follow, after a rules change on the server, lets it through). Each start calls
`notifyAppReady` once the app has mounted; a bundle that does not get that far within 20 s is
rolled back to the previous one. Installing a new APK drops downloaded bundles and starts from
the one inside the APK (`resetWhenUpdate`). Plain-http servers get no OTA bundles: the bundle is
code, and the plugin downloads over https only.

Settings shows which parts run: "Version 2.0.0 (web 512, app 498)" means web bundle 512 in the
shell of build 498.

**Trying it on an emulator.** Bundles download over https only, so a local test needs a
throwaway shell: set `httpsOnly: false` in the `CapacitorUpdater` config and drop the https
condition in `planUpdate`, build it with `MG_BUILD_NUMBER=<installed build>` (`adb install -r`
accepts the same versionCode), build again with a higher `MG_BUILD_NUMBER`, and serve that
build's `web-<build>.zip` and `web.json` under `/downloads/` with CORS from the host; the
emulator reaches it as `ws://10.0.2.2:<port>/ws` in Settings. `adb logcat -s CapgoUpdater`
shows the download, the switch on background and `notifyAppReady`. Finish by installing a real
APK with a higher build: the new versionCode drops the test bundle.

**Forcing an APK.** Change anything native (a file under `android/`, `capacitor.config.ts`, a
Capacitor or Capgo plugin version): the fingerprint changes, and apps with the old shell are
offered the APK instead of the bundle.

## Releases

To release, bump `[workspace.package] version` in `Cargo.toml` and push to `master`. Once CI
passes on that commit, `.github/workflows/release.yml` finds no `v<version>` tag for it, builds
the APK (with `web.json` and the over-the-air zip), the musl server binary and the server-hosted
web bundle, and publishes them as the GitHub Release `v<version>`, tagging that commit. A red or
cancelled CI publishes nothing; the next green push releases the version. Pushes that keep the
version build nothing. A manual run of the workflow builds the same files as artifacts without publishing.

Repository secrets the workflow needs:

| Secret | Value |
|---|---|
| `ANDROID_RELEASE_KEYSTORE_BASE64` | `base64 -w0 .keys/mini-games-release.jks` |
| `ANDROID_RELEASE_STORE_PASSWORD` | `storePassword` from `.keys/release-signing.properties` |
| `ANDROID_RELEASE_KEY_ALIAS` | `keyAlias` from the same file |
| `ANDROID_RELEASE_KEY_PASSWORD` | `keyPassword` from the same file |
