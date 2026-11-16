import { isNativeApp } from "@/platform/native-app";

/** Touch-first device (the app, phones, tablets): hints talk about fingers, not keys. */
export function prefersTouch(): boolean {
  return isNativeApp || (typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches);
}
