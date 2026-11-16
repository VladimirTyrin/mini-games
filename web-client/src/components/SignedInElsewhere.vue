<script setup lang="ts">
import { useConnectionStore } from "@/stores/connection";

const connection = useConnectionStore();
</script>

<template>
  <div
    v-if="connection.state === 'replaced'"
    class="fixed inset-x-0 top-0 z-40 flex items-center justify-center gap-3 bg-slate-800/95 px-4 py-3 text-sm shadow-lg"
    style="padding-top: max(0.75rem, env(safe-area-inset-top))"
    role="status"
  >
    <span class="text-gray-200">You're playing on another device.</span>
    <button class="rounded-full bg-emerald-600 px-4 py-2 font-semibold" @click="connection.continueHere()">
      Continue here
    </button>
  </div>
</template>
