<script setup lang="ts">
import { t } from "@/i18n";
import { usePauseControl } from "@/match/pause-control";

const pause = usePauseControl();
</script>

<template>
  <!-- Beside a board's own buttons on a touch screen, where the thumb is; the header's is out of
       reach on a tall phone. Resuming is the veil's button. -->
  <button
    v-if="pause && pause.canPause.value && !pause.paused.value"
    type="button"
    class="flex items-center justify-center rounded-full border border-white/10 bg-panel-raised text-lg text-gray-300 shadow-md active:scale-95"
    :aria-label="t.match.pause"
    data-testid="pause-button"
    @click="pause.toggle()"
  >
    {{ "⏸" }}
  </button>
</template>
