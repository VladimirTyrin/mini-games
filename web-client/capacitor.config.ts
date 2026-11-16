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
  },
};

export default config;
