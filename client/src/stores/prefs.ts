import { fromJsonString, toJsonString } from "@bufbuild/protobuf";
import { defineStore } from "pinia";
import { ref, watch } from "vue";

import { type GameSettings, GameSettingsSchema } from "@/proto/minigames/v2/game_pb";
import { defaultServerUrl } from "@/platform/server-url";
import { storage } from "@/platform/storage";

const KEY = "mini-games.prefs.v2";

interface StoredPrefs {
  serverUrl?: string;
  guestName?: string;
  sessionToken?: string;
  /** Protobuf JSON of the last settings used per game id. */
  settings?: Record<string, string>;
  debugMessages?: boolean;
}

function load(): StoredPrefs {
  const raw = storage.get(KEY);
  if (!raw) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? (parsed as StoredPrefs) : {};
  } catch {
    return {};
  }
}

/** Device-local preferences. */
export const usePrefsStore = defineStore("prefs", () => {
  const stored = load();
  const serverUrl = ref(stored.serverUrl ?? defaultServerUrl());
  const guestName = ref(stored.guestName ?? "");
  /** Signed session from a Google sign-in; lets the app skip Google on the next start. */
  const sessionToken = ref(stored.sessionToken ?? "");
  const settingsJson = ref<Record<string, string>>(stored.settings ?? {});
  const debugMessages = ref(stored.debugMessages ?? false);

  watch(
    [serverUrl, guestName, sessionToken, settingsJson, debugMessages],
    () => {
      const next: StoredPrefs = {
        serverUrl: serverUrl.value,
        guestName: guestName.value,
        sessionToken: sessionToken.value,
        settings: settingsJson.value,
        debugMessages: debugMessages.value,
      };
      storage.set(KEY, JSON.stringify(next));
    },
    { deep: true },
  );

  function lastSettings(gameId: string): GameSettings | null {
    const json = settingsJson.value[gameId];
    if (!json) return null;
    try {
      return fromJsonString(GameSettingsSchema, json);
    } catch {
      return null;
    }
  }

  function rememberSettings(gameId: string, settings: GameSettings): void {
    settingsJson.value = { ...settingsJson.value, [gameId]: toJsonString(GameSettingsSchema, settings) };
  }

  function resetServerUrl(): void {
    serverUrl.value = defaultServerUrl();
  }

  return {
    serverUrl,
    guestName,
    sessionToken,
    debugMessages,
    lastSettings,
    rememberSettings,
    resetServerUrl,
  };
});
