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

The APK lands in `android/app/build/outputs/apk/<variant>/`. `build/android.mjs` builds the web
bundle with `VITE_PLATFORM=android`: only that flavour contains the Capacitor plugins, the web
build drops them at build time. The default server is `wss://braintvsminigames.xyz/ws`; override it with
`MG_ANDROID_SERVER_URL`. Users can change it in Settings (plain `ws://` to an IP is allowed;
nginx serves `/ws` on port 80 too).

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
