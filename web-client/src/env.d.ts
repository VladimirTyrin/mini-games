/// <reference types="vite/client" />

declare const __APP_VERSION__: string;
declare const __NATIVE_APP__: boolean;

interface ImportMetaEnv {
  readonly VITE_SERVER_URL?: string;
  readonly VITE_BASE_PATH?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
