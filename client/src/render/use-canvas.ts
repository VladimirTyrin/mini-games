import { onBeforeUnmount, onMounted, type Ref, ref } from "vue";

export interface CanvasSize {
  /** CSS pixels. */
  width: number;
  height: number;
}

/**
 * A crisp canvas that tracks its container: the backing store is scaled by devicePixelRatio and
 * the context is pre-scaled, so drawing code uses CSS pixels. `draw` runs on demand
 * (`invalidate`) or every animation frame while `animating()` returns true — no idle rAF loop.
 */
export function useCanvas(
  canvas: Ref<HTMLCanvasElement | null>,
  container: Ref<HTMLElement | null>,
  draw: (context: CanvasRenderingContext2D, size: CanvasSize, now: number) => void,
  animating: () => boolean = () => false,
): { size: Ref<CanvasSize>; invalidate: () => void } {
  const size = ref<CanvasSize>({ width: 0, height: 0 });
  let frame: number | null = null;
  let observer: ResizeObserver | null = null;

  const render = (now: number) => {
    frame = null;
    const target = canvas.value;
    const context = target?.getContext("2d");
    if (!target || !context) return;
    const ratio = window.devicePixelRatio || 1;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    draw(context, size.value, now);
    if (animating()) invalidate();
  };

  function invalidate(): void {
    if (frame === null) frame = requestAnimationFrame(render);
  }

  const resize = () => {
    const target = canvas.value;
    const box = container.value;
    if (!target || !box) return;
    const ratio = window.devicePixelRatio || 1;
    const width = Math.floor(box.clientWidth);
    const height = Math.floor(box.clientHeight);
    size.value = { width, height };
    target.width = Math.max(1, Math.round(width * ratio));
    target.height = Math.max(1, Math.round(height * ratio));
    target.style.width = `${width}px`;
    target.style.height = `${height}px`;
    invalidate();
  };

  onMounted(() => {
    observer = new ResizeObserver(resize);
    if (container.value) observer.observe(container.value);
    resize();
  });
  onBeforeUnmount(() => {
    observer?.disconnect();
    if (frame !== null) cancelAnimationFrame(frame);
  });

  return { size, invalidate };
}
