import { onBeforeUnmount, type Ref, watch } from "vue";

export type SwipeDirection = "up" | "down" | "left" | "right";

const MIN_DISTANCE_PX = 24;

/**
 * Swipe and tap gestures on an element. Sets `touch-action: none` so the browser never scrolls
 * or pull-to-refreshes the page mid-game (which on Android reloads and loses the session).
 */
export function useSwipe(
  element: Ref<HTMLElement | null>,
  handlers: { swipe?: (direction: SwipeDirection) => void; tap?: (event: PointerEvent) => void },
): void {
  let start: { x: number; y: number; id: number } | null = null;

  const down = (event: PointerEvent) => {
    start = { x: event.clientX, y: event.clientY, id: event.pointerId };
  };
  const up = (event: PointerEvent) => {
    if (!start || start.id !== event.pointerId) return;
    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    start = null;
    if (Math.max(Math.abs(dx), Math.abs(dy)) < MIN_DISTANCE_PX) {
      handlers.tap?.(event);
      return;
    }
    if (Math.abs(dx) > Math.abs(dy)) handlers.swipe?.(dx > 0 ? "right" : "left");
    else handlers.swipe?.(dy > 0 ? "down" : "up");
  };
  const cancel = () => (start = null);

  let attached: HTMLElement | null = null;
  const detach = () => {
    if (!attached) return;
    attached.removeEventListener("pointerdown", down);
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
      target.addEventListener("pointerup", up);
      target.addEventListener("pointercancel", cancel);
      attached = target;
    },
    { immediate: true },
  );
  onBeforeUnmount(detach);
}
