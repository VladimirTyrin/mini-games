<script setup lang="ts">
import { nextTick, ref, watch } from "vue";

import type { ChatMessage } from "@/proto/minigames/v2/lobby_pb";

const props = defineProps<{ messages: ChatMessage[] }>();
const emit = defineEmits<{ send: [text: string] }>();
const text = ref("");
const list = ref<HTMLElement | null>(null);

function send(): void {
  const trimmed = text.value.trim();
  if (!trimmed) return;
  emit("send", trimmed);
  text.value = "";
}

watch(
  () => props.messages.length,
  async () => {
    await nextTick();
    list.value?.scrollTo({ top: list.value.scrollHeight });
  },
);
</script>

<template>
  <section class="flex min-h-0 flex-col rounded-xl bg-panel">
    <ol ref="list" class="max-h-56 min-h-24 flex-1 space-y-1 overflow-y-auto p-3 text-sm">
      <li v-for="(message, index) in messages" :key="index">
        <span class="font-semibold text-emerald-300">{{ message.sender?.displayName }}:</span>
        {{ message.text }}
      </li>
      <li v-if="!messages.length" class="text-gray-500">No messages yet</li>
    </ol>
    <form class="flex gap-2 border-t border-white/5 p-2" @submit.prevent="send" @keydown.stop>
      <input
        v-model="text"
        maxlength="300"
        placeholder="Say something"
        class="flex-1 rounded bg-panel-raised px-2 py-1 text-sm"
      />
      <button class="rounded bg-panel-raised px-3 text-sm" :disabled="!text.trim()">Send</button>
    </form>
  </section>
</template>
