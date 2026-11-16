import { onBeforeUnmount, onMounted } from "vue";

/** Returns `true` when the key was used (default action is then prevented). */
export type KeyHandler = (key: string, event: KeyboardEvent) => boolean;

export interface GameKeyOptions {
  allowRepeat?: boolean;
  /** Key released; also called for keys pressed while typing, so a hold can always end. */
  release?: (key: string) => void;
  /** The window lost focus: keyups will not arrive, treat every key as released. */
  releaseAll?: () => void;
}

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

/** Enter and Space press a focused button or follow a focused link; that is all they do then. */
function pressesFocusedControl(target: EventTarget | null, key: string): boolean {
  if (key !== "Enter" && key !== " ") return false;
  return target instanceof HTMLElement && target.closest("button, a[href], [role='button']") !== null;
}

/**
 * Game keyboard input with the filters every board needs: ignores auto-repeat, modifier
 * shortcuts (Ctrl+F must stay browser find) and keys typed into inputs such as the chat.
 */
export function useGameKeys(handler: KeyHandler, options: GameKeyOptions = {}): void {
  const down = (event: KeyboardEvent) => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.repeat && !options.allowRepeat) return;
    if (isTyping(event.target) || pressesFocusedControl(event.target, event.key)) return;
    if (handler(event.key, event)) event.preventDefault();
  };
  const up = (event: KeyboardEvent) => options.release?.(event.key);
  const blur = () => options.releaseAll?.();
  onMounted(() => {
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("keydown", down);
    window.removeEventListener("keyup", up);
    window.removeEventListener("blur", blur);
  });
}
