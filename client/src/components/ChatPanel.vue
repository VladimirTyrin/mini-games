<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";

import Avatar from "@/components/Avatar.vue";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import type { ChatMessage } from "@/proto/minigames/v2/lobby_pb";
import { useConnectionStore } from "@/stores/connection";
import { playerLabels, sameIdentity } from "@/util/players";

/**
 * `players`: who is shown elsewhere on screen (lobby members), so chat labels match theirs.
 * `growsToFill`: the panel takes the height its parent leaves (a flex column) instead of a few lines.
 */
const props = defineProps<{
  messages: ChatMessage[];
  players?: readonly (Identity | undefined)[];
  growsToFill?: boolean;
}>();
const emit = defineEmits<{ send: [text: string] }>();
const text = ref("");
const list = ref<HTMLElement | null>(null);
const connection = useConnectionStore();

const labels = computed(() =>
  playerLabels([...(props.players ?? []), ...props.messages.map((message) => message.sender)]),
);
const isMine = (message: ChatMessage) => sameIdentity(message.sender, connection.identity);
function senderLabel(message: ChatMessage): string {
  const label = (message.sender && labels.value.get(message.sender.id)) ?? "";
  return isMine(message) ? `${label} (you)` : label;
}

function send(): void {
  const trimmed = text.value.trim();
  if (!trimmed) return;
  emit("send", trimmed);
  text.value = "";
}

// The newest message, not the count: once the history is full the count stays the same.
watch(
  () => props.messages.at(-1),
  async () => {
    await nextTick();
    list.value?.scrollTo({ top: list.value.scrollHeight });
  },
);
</script>

<template>
  <section class="flex min-h-0 flex-col rounded-xl bg-panel">
    <ol
      ref="list"
      class="flex-1 space-y-1 overflow-y-auto p-3 text-sm"
      :class="growsToFill ? 'min-h-10' : 'max-h-56 min-h-24'"
    >
      <li v-for="(message, index) in messages" :key="index" class="flex items-start gap-2">
        <Avatar
          v-if="message.sender"
          :url="message.sender.avatarUrl"
          :name="message.sender.displayName"
          :player-id="message.sender.id"
        />
        <span class="min-w-0 break-words">
          <span class="font-semibold" :class="isMine(message) ? 'text-sky-300' : 'text-emerald-300'"
            >{{ senderLabel(message) }}:</span
          >
          {{ message.text }}
        </span>
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
