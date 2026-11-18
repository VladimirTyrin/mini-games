<script setup lang="ts">
import { useBackHandler } from "@/platform/back-navigation";
import { usePageScrollLock } from "@/util/scroll-lock";

defineProps<{ title: string; message: string; confirmLabel: string; cancelLabel: string }>();
const emit = defineEmits<{ confirm: []; cancel: [] }>();

// The system back button closes the dialog instead of the screen behind it.
useBackHandler(() => emit("cancel"));
usePageScrollLock();
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex touch-none items-end justify-center overscroll-none bg-black/60 p-4 sm:items-center"
    role="dialog"
    aria-modal="true"
    @click.self="emit('cancel')"
  >
    <div class="w-full max-w-sm space-y-4 rounded-2xl bg-panel p-5 shadow-xl">
      <div class="space-y-1">
        <h2 class="text-lg font-semibold">{{ title }}</h2>
        <p class="text-sm text-gray-300">{{ message }}</p>
      </div>
      <div class="flex gap-2">
        <button class="h-11 flex-1 rounded-xl bg-panel-raised font-medium" @click="emit('cancel')">
          {{ cancelLabel }}
        </button>
        <button class="h-11 flex-1 rounded-xl bg-rose-700 font-semibold" @click="emit('confirm')">
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
