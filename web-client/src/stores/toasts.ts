import { defineStore } from "pinia";
import { ref } from "vue";

export type ToastTone = "info" | "success" | "error";

export interface Toast {
  id: number;
  tone: ToastTone;
  text: string;
}

const VISIBLE_MS = 4_000;
const MAX_TOASTS = 4;

export const useToastStore = defineStore("toasts", () => {
  const toasts = ref<Toast[]>([]);
  let nextId = 1;

  function show(text: string, tone: ToastTone = "info"): void {
    const toast = { id: nextId++, tone, text };
    toasts.value = [...toasts.value.slice(-(MAX_TOASTS - 1)), toast];
    setTimeout(() => dismiss(toast.id), VISIBLE_MS);
  }

  function dismiss(id: number): void {
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  return { toasts, show, dismiss, error: (text: string) => show(text, "error") };
});
