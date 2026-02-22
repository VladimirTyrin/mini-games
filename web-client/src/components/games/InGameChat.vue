<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue";
import { useChatStore, type ChatMessage } from "../../stores/chat";
import { useConnectionStore } from "../../stores/connection";

const chatStore = useChatStore();
const connectionStore = useConnectionStore();

const isExpanded = ref(false);
const chatInput = ref("");
const chatContainer = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);

const messages = computed(() => chatStore.inLobbyMessages);
const unreadCount = ref(0);

function formatPlayerName(message: ChatMessage): string {
  const name = message.sender.playerId;
  if (message.sender.isBot) return `[BOT] ${name}`;
  if (name === connectionStore.clientId) return "You";
  return name;
}

function senderClass(message: ChatMessage): string {
  if (message.sender.playerId === connectionStore.clientId) return "text-green-400";
  if (message.sender.isBot) return "text-purple-400";
  return "text-blue-400";
}

function toggleChat(): void {
  isExpanded.value = !isExpanded.value;
  if (isExpanded.value) {
    unreadCount.value = 0;
    nextTick(() => {
      scrollToBottom();
      inputRef.value?.focus();
    });
  }
}

function sendMessage(): void {
  if (chatInput.value.trim()) {
    chatStore.sendInLobbyMessage(chatInput.value.trim());
    chatInput.value = "";
  }
}

function scrollToBottom(): void {
  nextTick(() => {
    if (chatContainer.value) {
      chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
    }
  });
}

function handleKeyDown(event: KeyboardEvent): void {
  if (event.key === "Enter" && !isExpanded.value) {
    event.preventDefault();
    event.stopPropagation();
    toggleChat();
  } else if (event.key === "Escape" && isExpanded.value) {
    event.preventDefault();
    event.stopPropagation();
    isExpanded.value = false;
  }
}

watch(messages, () => {
  if (isExpanded.value) {
    scrollToBottom();
  } else {
    unreadCount.value++;
  }
}, { deep: true });
</script>

<template>
  <div class="absolute top-2 right-2 z-10 flex flex-col items-end">
    <button
      class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors flex items-center gap-1.5"
      :class="isExpanded
        ? 'bg-blue-600 text-white'
        : 'bg-gray-800/80 hover:bg-gray-700/80 text-gray-300 backdrop-blur-sm'"
      @click="toggleChat"
      @keydown="handleKeyDown"
    >
      Chat
      <span
        v-if="unreadCount > 0 && !isExpanded"
        class="bg-red-500 text-white text-xs rounded-full w-5 h-5 flex items-center justify-center"
      >
        {{ unreadCount > 9 ? "9+" : unreadCount }}
      </span>
    </button>

    <div
      v-if="isExpanded"
      class="mt-1 w-72 bg-gray-800/95 backdrop-blur-sm border border-gray-700 rounded-lg shadow-lg flex flex-col"
      style="max-height: 300px;"
    >
      <div
        ref="chatContainer"
        class="flex-1 overflow-y-auto p-2 space-y-1 min-h-0"
        style="max-height: 240px;"
      >
        <div
          v-for="(msg, index) in messages"
          :key="index"
          class="text-xs"
        >
          <span class="font-medium" :class="senderClass(msg)">{{ formatPlayerName(msg) }}:</span>
          <span class="text-gray-300"> {{ msg.message }}</span>
        </div>
        <div
          v-if="messages.length === 0"
          class="text-gray-500 text-center py-4 text-xs"
        >
          No messages yet
        </div>
      </div>

      <div class="p-2 border-t border-gray-700">
        <input
          ref="inputRef"
          v-model="chatInput"
          type="text"
          placeholder="Type a message..."
          class="w-full px-2 py-1.5 bg-gray-700 rounded border border-gray-600 text-sm text-white placeholder-gray-400 focus:border-blue-500 focus:outline-none"
          @keyup.enter="sendMessage"
          @keydown.stop
        />
      </div>
    </div>
  </div>
</template>
