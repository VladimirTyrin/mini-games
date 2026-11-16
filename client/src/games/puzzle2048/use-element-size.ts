import { onBeforeUnmount, type Ref, ref, watch } from "vue";

export interface ElementSize {
  readonly width: number;
  readonly height: number;
}

/** Content-box size of an element, kept current with a `ResizeObserver`. */
export function useElementSize(element: Ref<HTMLElement | null>): Ref<ElementSize> {
  const size = ref<ElementSize>({ width: 0, height: 0 });
  const observer = new ResizeObserver(([entry]) => {
    if (entry) size.value = { width: entry.contentRect.width, height: entry.contentRect.height };
  });
  watch(
    element,
    (target, previous) => {
      if (previous) observer.unobserve(previous);
      if (target) observer.observe(target);
    },
    { immediate: true },
  );
  onBeforeUnmount(() => observer.disconnect());
  return size;
}
