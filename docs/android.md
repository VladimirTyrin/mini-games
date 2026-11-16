# Android app

The app is the web client inside Capacitor 8 (`web-client/android`). Games, bots and replays run
on the same WASM core as in the browser; the app adds the system back button, native Google
sign-in and an editable server address (Settings).

## Build locally

Needs JDK 21 and the Android SDK (platform 36, build-tools 36) with `JAVA_HOME` and
`ANDROID_HOME` set, plus the usual web toolchain (see `AGENTS.md`).

```bash
cd web-client
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

## Updates

`./deploy.sh --apk web-client/android/dist/mini-games.apk` publishes the APK at
`/downloads/mini-games.apk` and its `android.json`. On start (and after the server address
changes) the app reads `android.json` from its server and, when the build is newer, offers
"Update available → Download" on the home and settings screens; the system browser downloads
the APK and Android offers to install it.

## Releases

Pushing a tag `v<workspace version>` (e.g. `v2.0.0`) runs `.github/workflows/release.yml`, which
publishes a GitHub Release with the APK, the musl server binary and the server-hosted web bundle.
A manual run of the workflow builds the same files as artifacts without publishing.

Repository secrets the workflow needs:

| Secret | Value |
|---|---|
| `ANDROID_RELEASE_KEYSTORE_BASE64` | `base64 -w0 .keys/mini-games-release.jks` |
| `ANDROID_RELEASE_STORE_PASSWORD` | `storePassword` from `.keys/release-signing.properties` |
| `ANDROID_RELEASE_KEY_ALIAS` | `keyAlias` from the same file |
| `ANDROID_RELEASE_KEY_PASSWORD` | `keyPassword` from the same file |
