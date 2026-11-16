import { handleBack } from "./back-navigation";
import { confirmBundleStarted } from "./web-bundle";

/** Runs once the app has mounted. */
export async function setUpNativeApp(): Promise<void> {
  if (!__NATIVE_APP__) return;
  try {
    await confirmBundleStarted();
  } catch {
    // The app keeps working; at worst the plugin rolls a freshly switched bundle back.
  }
  const { App } = await import("@capacitor/app");
  await App.addListener("backButton", () => {
    // On the home screen Android expects back to leave the app; minimizing keeps a running
    // online session instead of killing it.
    if (!handleBack()) void App.minimizeApp();
  });
}
