import type { CapacitorConfig } from "@capacitor/cli";

const config: CapacitorConfig = {
  appId: "xyz.braintvsminigames.app",
  appName: "Mini Games",
  webDir: "dist",
  backgroundColor: "#111827",
  server: {
    // The server address is user-editable and may be a bare IP over plain ws://.
    cleartext: true,
  },
  android: {
    // The app is served from https://localhost; without this the WebView blocks ws:// sockets.
    allowMixedContent: true,
  },
  plugins: {
    // Light icons on the app's dark background.
    SystemBars: { style: "DARK" },
    SocialLogin: {
      // Only Google; the other providers would bundle their SDKs into the APK.
      providers: { google: true, facebook: false, apple: false, twitter: false },
      logLevel: 1,
    },
    // Over-the-air web bundles from our own server, driven by src/platform/app-update.ts:
    // no Capgo cloud, no update checks of its own, no statistics.
    CapacitorUpdater: {
      autoUpdate: false,
      updateUrl: "",
      channelUrl: "",
      statsUrl: "",
      // A new APK starts from the bundle inside it, dropping bundles downloaded for the old shell.
      resetWhenUpdate: true,
      // Keeps a bundle that failed to start in the list, so the same build is not tried again.
      autoDeleteFailed: false,
      // Time from start to notifyAppReady (after WASM load and mount) before rolling back.
      appReadyTimeout: 20000,
    },
  },
};

export default config;
