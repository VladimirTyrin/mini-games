/** Touch-first device (the app, phones, tablets): hints talk about fingers, not keys. */
export function prefersTouch(): boolean {
  return __NATIVE_APP__ || (typeof matchMedia === "function" && matchMedia("(pointer: coarse)").matches);
}
