import { onBeforeUnmount, onMounted } from "vue";

/** Returns `true` when the key was used (default action is then prevented). */
export type KeyHandler = (key: string, event: KeyboardEvent) => boolean;

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

/**
 * Game keyboard input with the filters every board needs: ignores auto-repeat, modifier
 * shortcuts (Ctrl+F must stay browser find) and keys typed into inputs such as the chat.
 */
export function useGameKeys(handler: KeyHandler, options: { allowRepeat?: boolean } = {}): void {
  const listener = (event: KeyboardEvent) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.repeat && !options.allowRepeat) return;
    if (isTyping(event.target)) return;
    if (handler(event.key, event)) event.preventDefault();
  };
  onMounted(() => window.addEventListener("keydown", listener));
  onBeforeUnmount(() => window.removeEventListener("keydown", listener));
}
