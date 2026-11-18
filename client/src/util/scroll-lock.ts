import { onBeforeUnmount, onMounted } from "vue";

let holders = 0;

/**
 * Keeps the page still while a dialog over it is open. A swipe that reached the end of the
 * dialog's own text used to scroll the page instead; on a phone that shows or hides the address
 * bar, and every size taken from `dvh` (the boards) jumps with it.
 */
export function lockPageScroll(): () => void {
  holders += 1;
  if (holders === 1) document.documentElement.style.overflow = "hidden";
  let released = false;
  return () => {
    if (released) return;
    released = true;
    holders -= 1;
    if (holders === 0) document.documentElement.style.overflow = "";
  };
}

/** `lockPageScroll` for the lifetime of a component. */
export function usePageScrollLock(): void {
  let release: (() => void) | null = null;
  onMounted(() => (release = lockPageScroll()));
  onBeforeUnmount(() => release?.());
}
