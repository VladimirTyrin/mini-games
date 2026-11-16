<script setup lang="ts">
import { useRouter } from "vue-router";

import { type Toast, useToastStore } from "@/stores/toasts";

const router = useRouter();
const toasts = useToastStore();
const toneClass = { info: "bg-slate-700", success: "bg-emerald-700", error: "bg-rose-700" } as const;

function runAction(toast: Toast): void {
  toasts.dismiss(toast.id);
  if (toast.action) void router.push(toast.action.to);
}
</script>

<template>
  <div
    class="pointer-events-none fixed inset-x-0 z-50 flex flex-col items-center gap-2 px-4"
    style="bottom: calc(1rem + env(safe-area-inset-bottom))"
  >
    <div
      v-for="toast in toasts.toasts"
      :key="toast.id"
      class="pointer-events-auto flex max-w-md items-center gap-3 rounded-lg text-sm shadow-lg"
      :class="toneClass[toast.tone]"
    >
      <button
        class="flex-1 px-4 py-2"
        :class="{ 'text-left': toast.action }"
        @click="toasts.dismiss(toast.id)"
      >
        {{ toast.text }}
      </button>
      <button
        v-if="toast.action"
        class="mr-2 shrink-0 rounded bg-black/25 px-3 py-1 font-semibold hover:bg-black/40"
        @click="runAction(toast)"
      >
        {{ toast.action.label }}
      </button>
    </div>
  </div>
</template>
