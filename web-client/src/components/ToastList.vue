<script setup lang="ts">
import { useToastStore } from "@/stores/toasts";

const toasts = useToastStore();
const toneClass = { info: "bg-slate-700", success: "bg-emerald-700", error: "bg-rose-700" } as const;
</script>

<template>
  <div class="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex flex-col items-center gap-2 px-4">
    <button
      v-for="toast in toasts.toasts"
      :key="toast.id"
      class="pointer-events-auto max-w-md rounded-lg px-4 py-2 text-sm shadow-lg"
      :class="toneClass[toast.tone]"
      @click="toasts.dismiss(toast.id)"
    >
      {{ toast.text }}
    </button>
  </div>
</template>
