import { defineStore } from "pinia";
import { ref } from "vue";
import type {
  LobbyListChatNotification,
  InLobbyChatNotification,
  PlayerIdentity,
} from "../proto/game_service_pb";
import { gameClient } from "../api/client";

export interface ChatMessage {
  sender: PlayerIdentity;
  message: string;
  timestamp: number;
}

export const useChatStore = defineStore("chat", () => {
  const lobbyListMessages = ref<ChatMessage[]>([]);
  const inLobbyMessages = ref<ChatMessage[]>([]);

  function sendLobbyListMessage(text: string): void {
    gameClient.sendChat(text, false);
  }

  function sendInLobbyMessage(text: string): void {
    gameClient.sendChat(text, true);
  }

  function sendMessage(text: string, inLobby: boolean): void {
    gameClient.sendChat(text, inLobby);
  }

  function handleLobbyListChat(notification: LobbyListChatNotification): void {
    if (!notification.sender) return;

    lobbyListMessages.value.push({
      sender: notification.sender,
      message: notification.message,
      timestamp: Date.now(),
    });
  }

  function handleInLobbyChat(notification: InLobbyChatNotification): void {
    if (!notification.sender) return;

    inLobbyMessages.value.push({
      sender: notification.sender,
      message: notification.message,
      timestamp: Date.now(),
    });
  }

  function clearLobbyListMessages(): void {
    lobbyListMessages.value = [];
  }

  function clearInLobbyMessages(): void {
    inLobbyMessages.value = [];
  }

  function clearAll(): void {
    clearLobbyListMessages();
    clearInLobbyMessages();
  }

  return {
    lobbyListMessages,
    inLobbyMessages,
    sendLobbyListMessage,
    sendInLobbyMessage,
    sendMessage,
    handleLobbyListChat,
    handleInLobbyChat,
    clearLobbyListMessages,
    clearInLobbyMessages,
    clearAll,
  };
});
