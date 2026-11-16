import { onBeforeUnmount, onMounted } from "vue";

const handlers: Array<() => void> = [];

/** Makes the system back button (Android) do what the screen's own back button does. */
export function useBackHandler(handler: () => void): void {
  onMounted(() => handlers.push(handler));
  onBeforeUnmount(() => {
    const index = handlers.lastIndexOf(handler);
    if (index >= 0) handlers.splice(index, 1);
  });
}

/** Runs the innermost screen's back action; false when there is none (home screen). */
export function handleBack(): boolean {
  const handler = handlers.at(-1);
  if (!handler) return false;
  handler();
  return true;
}
