<script setup lang="ts">
import { useToastStore, type ToastType } from "../../stores/toast";

const toastStore = useToastStore();

function getColorClasses(type: ToastType): string {
  const colors: Record<ToastType, string> = {
    error: "bg-red-600 border-red-500",
    success: "bg-green-600 border-green-500",
    warning: "bg-yellow-600 border-yellow-500",
    info: "bg-blue-600 border-blue-500",
  };
  return colors[type];
}

function getIcon(type: ToastType): string {
  const icons: Record<ToastType, string> = {
    error: "!",
    success: "\u2713",
    warning: "\u26A0",
    info: "i",
  };
  return icons[type];
}
</script>

<template>
  <Teleport to="body">
    <div class="fixed bottom-4 right-4 z-50 space-y-2 max-w-sm">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toastStore.toasts"
          :key="toast.id"
          :class="[
            getColorClasses(toast.type),
            'px-4 py-3 rounded-lg shadow-lg text-white border flex items-start gap-3',
          ]"
        >
          <span class="flex-shrink-0 w-5 h-5 rounded-full bg-white/20 flex items-center justify-center text-xs font-bold">
            {{ getIcon(toast.type) }}
          </span>
          <span class="flex-1 text-sm">{{ toast.message }}</span>
          <button
            class="flex-shrink-0 text-white/70 hover:text-white transition-colors"
            @click="toastStore.remove(toast.id)"
          >
            <span class="text-lg leading-none">&times;</span>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>
