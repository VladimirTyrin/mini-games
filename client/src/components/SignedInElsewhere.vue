<script setup lang="ts">
// Each device or tab is its own player; this only shows when the very same session (a
// duplicated tab carries its resume token) was opened in another window and took it over.
import { t } from "@/i18n";
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
    <span class="text-gray-200">{{ t.app.signedInElsewhere.openInAnotherWindow }}</span>
    <button
      class="rounded-full bg-emerald-600 px-4 py-2 font-semibold whitespace-nowrap"
      @click="connection.continueHere()"
    >
      {{ t.app.signedInElsewhere.playHere }}
    </button>
  </div>
</template>
