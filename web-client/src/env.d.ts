/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

declare const __APP_VERSION__: string;
/**
 * True only in the Android build (`VITE_PLATFORM=android`, see vite.config.ts). Gate native code
 * with this literal directly: the bundler drops `if (!__NATIVE_APP__)` branches, but not
 * branches on a constant re-exported from another module.
 */
declare const __NATIVE_APP__: boolean;
declare const __APP_BUILD__: number;
/** Android build only: fingerprint of the native shell the bundle was built for; "" on the web. */
declare const __NATIVE_FINGERPRINT__: string;

interface ImportMetaEnv {
  readonly VITE_SERVER_URL?: string;
  readonly VITE_BASE_PATH?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
