import { defineStore } from "pinia";
import { ref } from "vue";
import { errorMessage } from "@/util/error-message";

export type ToastTone = "info" | "success" | "error";

/** A button on the toast that opens a screen ("Replay saved" → "Show"). */
export interface ToastAction {
  label: string;
  to: string;
}

export interface Toast {
  id: number;
  tone: ToastTone;
  text: string;
  action?: ToastAction;
}

const VISIBLE_MS = 4_000;
/** Long enough to read the text and still reach the button. */
const VISIBLE_WITH_ACTION_MS = 7_000;
const MAX_TOASTS = 4;

export const useToastStore = defineStore("toasts", () => {
  const toasts = ref<Toast[]>([]);
  let nextId = 1;

  function show(text: string, tone: ToastTone = "info", action?: ToastAction): void {
    const toast: Toast = { id: nextId++, tone, text, action };
    toasts.value = [...toasts.value.slice(-(MAX_TOASTS - 1)), toast];
    setTimeout(() => dismiss(toast.id), action ? VISIBLE_WITH_ACTION_MS : VISIBLE_MS);
  }

  function dismiss(id: number): void {
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  /** Shows a failure from any `catch`: the error's own message, or its text. */
  function failure(error: unknown): void {
    show(errorMessage(error), "error");
  }

  return { toasts, show, dismiss, failure, error: (text: string) => show(text, "error") };
});
