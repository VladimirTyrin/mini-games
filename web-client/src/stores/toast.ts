import { defineStore } from "pinia";
import { ref } from "vue";

export type ToastType = "error" | "success" | "warning" | "info";

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
  duration: number;
}

export const useToastStore = defineStore("toast", () => {
  const toasts = ref<Toast[]>([]);
  let nextId = 0;

  function show(
    message: string,
    type: ToastType = "info",
    duration = 5000
  ): void {
    const id = nextId++;
    toasts.value.push({ id, message, type, duration });

    if (duration > 0) {
      setTimeout(() => remove(id), duration);
    }
  }

  function remove(id: number): void {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  function error(message: string, duration = 5000): void {
    show(message, "error", duration);
  }

  function success(message: string, duration = 3000): void {
    show(message, "success", duration);
  }

  function warning(message: string, duration = 4000): void {
    show(message, "warning", duration);
  }

  function info(message: string, duration = 3000): void {
    show(message, "info", duration);
  }

  function clear(): void {
    toasts.value = [];
  }

  return {
    toasts,
    show,
    remove,
    error,
    success,
    warning,
    info,
    clear,
  };
});
