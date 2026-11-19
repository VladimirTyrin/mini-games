import { type Ref, onBeforeUnmount, watch } from "vue";

import { TwoFingerTap } from "./two-finger-tap";

function points(list: TouchList): { id: number; x: number; y: number }[] {
  return [...list].map((touch) => ({ id: touch.identifier, x: touch.clientX, y: touch.clientY }));
}

/** Calls `onTap` on a tap with two fingers anywhere on `area` (passive: one finger's input stays the board's). */
export function useTwoFingerTap(area: Ref<HTMLElement | null>, onTap: () => void): void {
  const tap = new TwoFingerTap();
  const start = (event: TouchEvent) => tap.down(points(event.touches), event.timeStamp);
  const move = (event: TouchEvent) => tap.moved(points(event.touches));
  const end = (event: TouchEvent) => {
    if (tap.up(points(event.touches), event.timeStamp)) onTap();
  };
  const listen = (element: HTMLElement, on: boolean) => {
    const method = on ? "addEventListener" : "removeEventListener";
    element[method]("touchstart", start as EventListener, { passive: true });
    element[method]("touchmove", move as EventListener, { passive: true });
    element[method]("touchend", end as EventListener, { passive: true });
    element[method]("touchcancel", end as EventListener, { passive: true });
  };
  watch(
    area,
    (element, previous) => {
      if (previous) listen(previous, false);
      if (element) listen(element, true);
    },
    { immediate: true },
  );
  onBeforeUnmount(() => {
    if (area.value) listen(area.value, false);
  });
}
