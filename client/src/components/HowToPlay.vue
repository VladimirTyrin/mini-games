<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";

import GameIcon from "@/components/GameIcon.vue";
import { type GamePlugin, controlsHintFor } from "@/games/game-plugin";
import { useBackHandler } from "@/platform/back-navigation";
import { markHelpSeen } from "@/platform/help-seen";

const props = defineProps<{ plugin: GamePlugin }>();
const emit = defineEmits<{ close: [] }>();

function close(): void {
  emit("close");
}

useBackHandler(close);
markHelpSeen(props.plugin.id);

function onKey(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  // Escape here closes the help, not the match's own pause behind it.
  event.preventDefault();
  event.stopImmediatePropagation();
  close();
}
onMounted(() => window.addEventListener("keydown", onKey, { capture: true }));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, { capture: true }));
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-end justify-center bg-gray-950/60 sm:items-center sm:p-4"
    role="dialog"
    aria-modal="true"
    :aria-label="`How to play ${props.plugin.title}`"
    @click.self="close"
  >
    <div
      class="flex max-h-[92dvh] w-full max-w-lg flex-col rounded-t-2xl bg-panel shadow-xl ring-1 ring-white/10 sm:rounded-2xl"
    >
      <header class="flex items-center gap-3 border-b border-white/5 px-4 py-3">
        <GameIcon :plugin="props.plugin" class="size-8 text-xl" />
        <h2 class="min-w-0 flex-1 text-lg font-semibold">How to play {{ props.plugin.title }}</h2>
        <button
          type="button"
          class="rounded px-2 py-1 text-lg hover:bg-panel-raised"
          aria-label="Close"
          @click="close"
        >
          ✕
        </button>
      </header>
      <div class="how-to-play min-h-0 flex-1 overflow-y-auto px-4 py-3">
        <component :is="props.plugin.Help" />
        <h3>Controls</h3>
        <p>{{ controlsHintFor(props.plugin) }}</p>
      </div>
      <div
        class="border-t border-white/5 p-3"
        style="padding-bottom: max(0.75rem, env(safe-area-inset-bottom))"
      >
        <button
          type="button"
          class="h-11 w-full rounded-xl bg-emerald-600 font-semibold hover:bg-emerald-500"
          @click="close"
        >
          Got it
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.how-to-play :deep(h3) {
  margin: 1rem 0 0.375rem;
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: #9ca3af;
}

.how-to-play :deep(h3:first-child) {
  margin-top: 0;
}

.how-to-play :deep(p),
.how-to-play :deep(li) {
  font-size: 0.9375rem;
  line-height: 1.45;
  color: #e5e7eb;
}

.how-to-play :deep(ul) {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding-left: 1.1rem;
  list-style: disc;
}

.how-to-play :deep(li::marker) {
  color: #6b7280;
}

.how-to-play :deep(b) {
  color: #fff;
  font-weight: 600;
}
</style>
