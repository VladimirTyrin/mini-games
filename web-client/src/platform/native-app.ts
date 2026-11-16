import { handleBack } from "./back-navigation";

export async function setUpNativeApp(): Promise<void> {
  if (!__NATIVE_APP__) return;
  const { App } = await import("@capacitor/app");
  await App.addListener("backButton", () => {
    // On the home screen Android expects back to leave the app; minimizing keeps a running
    // online session instead of killing it.
    if (!handleBack()) void App.minimizeApp();
  });
}
