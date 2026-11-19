<script setup lang="ts">
import { t } from "@/i18n";

const emit = defineEmits<{ open: [] }>();

// Clicked with the mouse (`detail` counts clicks; a key press has none), it lets the focus go:
// kept, it would take the next Space (Stack Attack's jump) and open the help again.
function open(event: MouseEvent): void {
  if (event.detail > 0 && event.currentTarget instanceof HTMLElement) event.currentTarget.blur();
  emit("open");
}
</script>

<template>
  <button
    type="button"
    class="flex size-8 shrink-0 items-center justify-center rounded-full border border-white/15 text-sm font-semibold text-gray-300 hover:bg-panel-raised hover:text-white"
    :aria-label="t.match.howToPlay.button"
    :title="t.match.howToPlay.button"
    data-testid="how-to-play"
    @click="open"
  >
    ?
  </button>
</template>
