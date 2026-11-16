import { handleBack } from "./back-navigation";

/** True only in the Android build (`VITE_PLATFORM=android`, see vite.config.ts). */
export const isNativeApp = __NATIVE_APP__;

export async function setUpNativeApp(): Promise<void> {
  if (!isNativeApp) return;
  const { App } = await import("@capacitor/app");
  await App.addListener("backButton", () => {
    // On the home screen Android expects back to leave the app; minimizing keeps a running
    // online session instead of killing it.
    if (!handleBack()) void App.minimizeApp();
  });
}
