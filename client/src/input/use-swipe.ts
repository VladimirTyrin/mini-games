import { onBeforeUnmount, type Ref, watch } from "vue";

import { type SwipeDirection, SwipeGesture, type SwipeMode } from "./swipe-gesture";

export type { SwipeDirection, SwipeMode } from "./swipe-gesture";

/**
 * Swipe and tap gestures on an element. Sets `touch-action: none` so the browser never scrolls
 * or pull-to-refreshes the page mid-game (which on Android reloads and loses the session).
 */
export function useSwipe(
  element: Ref<HTMLElement | null>,
  handlers: { swipe?: (direction: SwipeDirection) => void; tap?: (event: PointerEvent) => void },
  mode: SwipeMode = "on-release",
): void {
  const gesture = new SwipeGesture(mode);
  let fingerId: number | null = null;

  const down = (event: PointerEvent) => {
    fingerId = event.pointerId;
    gesture.down({ x: event.clientX, y: event.clientY });
    // A drag that leaves the board keeps steering instead of being lost at its edge.
    if (mode === "while-dragging") (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  };
  const move = (event: PointerEvent) => {
    if (event.pointerId !== fingerId) return;
    const direction = gesture.moved({ x: event.clientX, y: event.clientY });
    if (direction) handlers.swipe?.(direction);
  };
  const up = (event: PointerEvent) => {
    if (event.pointerId !== fingerId) return;
    fingerId = null;
    const outcome = gesture.up({ x: event.clientX, y: event.clientY });
    if (outcome.kind === "swipe") handlers.swipe?.(outcome.direction);
    else if (outcome.kind === "tap") handlers.tap?.(event);
  };
  const cancel = () => {
    fingerId = null;
    gesture.cancel();
  };

  let attached: HTMLElement | null = null;
  const detach = () => {
    if (!attached) return;
    attached.removeEventListener("pointerdown", down);
    attached.removeEventListener("pointermove", move);
    attached.removeEventListener("pointerup", up);
    attached.removeEventListener("pointercancel", cancel);
    attached = null;
  };
  watch(
    element,
    (target) => {
      detach();
      if (!target) return;
      target.style.touchAction = "none";
      target.addEventListener("pointerdown", down);
      target.addEventListener("pointermove", move);
      target.addEventListener("pointerup", up);
      target.addEventListener("pointercancel", cancel);
      attached = target;
    },
    { immediate: true },
  );
  onBeforeUnmount(detach);
}
