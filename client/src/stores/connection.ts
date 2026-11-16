import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";

import type { ServerInfo } from "@/proto/minigames/v2/auth_pb";
import type { Identity } from "@/proto/minigames/v2/identity_pb";
import type { Credentials } from "@/net/credentials";
import { probeServer } from "@/net/probe-server";
import { RequestError } from "@/net/request-error";
import { type ConnectionState, Session } from "@/net/session";
import { storage, tabStorage } from "@/platform/storage";

/**
 * Where the resume token lives. Every browser tab is its own player (several devices or tabs of
 * one account play side by side), so on the web it stays with the tab. The app has one window
 * that Android may kill in the background: there it is kept, so a restart gets the seat back.
 */
const resumeStorage = __NATIVE_APP__ ? storage : tabStorage;

import { usePrefsStore } from "./prefs";
import { useToastStore } from "./toasts";
import { errorMessage } from "@/util/error-message";

/** The online session: server info, sign-in state and the live `Session`. */
export const useConnectionStore = defineStore("connection", () => {
  const prefs = usePrefsStore();
  const toasts = useToastStore();

  const session = shallowRef<Session | null>(null);
  const state = ref<ConnectionState>("offline");
  const identity = ref<Identity | null>(null);
  const serverInfo = ref<ServerInfo | null>(null);
  const serverError = ref<string | null>(null);
  const isSignedIn = computed(() => state.value === "signed-in" && identity.value !== null);
  /** The name local games seat the player under: the account's when signed in, as on the cards. */
  const playerName = computed(() => identity.value?.displayName || prefs.guestName || "You");

  async function loadServerInfo(): Promise<void> {
    serverError.value = null;
    try {
      serverInfo.value = await probeServer(prefs.serverUrl);
    } catch (error) {
      serverInfo.value = null;
      serverError.value = errorMessage(error);
    }
  }

  /**
   * App start: signs in again with a stored Google session, or — after a reload of a tab that was
   * playing as a guest — with the same guest name, so the server hands the seat back.
   */
  function resumeStoredSession(): void {
    if (session.value) return;
    if (prefs.sessionToken) {
      start({ kind: "session", token: prefs.sessionToken });
    } else if (prefs.guestName && resumeStorage.get(resumeKeyFor(prefs.serverUrl))) {
      start({ kind: "guest", displayName: prefs.guestName });
    }
  }

  function resumeKeyFor(serverUrl: string): string {
    return `mini-games.resume.${serverUrl}`;
  }

  function start(credentials: Credentials): void {
    session.value?.close();
    const resumeKey = resumeKeyFor(prefs.serverUrl);
    const created = new Session(prefs.serverUrl, credentials, resumeStorage.get(resumeKey) ?? "");
    created.events.on("state", (next) => (state.value = next));
    created.events.on("signedIn", ({ identity: who, sessionToken, resumeToken, serverInfo: info }) => {
      identity.value = who;
      if (info) serverInfo.value = info;
      resumeStorage.set(resumeKey, resumeToken);
      // A Google sign-in returns a long-lived session token; later reconnects use it instead
      // of the short-lived Google ID token.
      if (sessionToken) {
        prefs.sessionToken = sessionToken;
        created.updateCredentials({ kind: "session", token: sessionToken });
      }
    });
    created.events.on("refused", (error) => {
      if (credentials.kind !== "session") {
        toasts.error(error.message);
        return;
      }
      // A stored session the server no longer accepts (expired, server secret or token format
      // changed): forget it and ask for a fresh sign-in instead of showing the raw error.
      prefs.sessionToken = "";
      toasts.show("Your session has expired. Please sign in again.");
    });
    created.events.on("shuttingDown", () => toasts.show("The server is restarting"));
    session.value = created;
    created.connect();
  }

  /** Takes the session back from the window that opened it after this one (a duplicated tab). */
  function continueHere(): void {
    if (prefs.sessionToken) start({ kind: "session", token: prefs.sessionToken });
    else if (prefs.guestName) start({ kind: "guest", displayName: prefs.guestName });
  }

  function signInAsGuest(name: string): void {
    prefs.guestName = name.trim();
    start({ kind: "guest", displayName: prefs.guestName });
  }

  function signInWithGoogle(idToken: string): void {
    start({ kind: "google", idToken });
  }

  async function signOut(): Promise<void> {
    resumeStorage.remove(resumeKeyFor(prefs.serverUrl));
    await session.value?.signOut();
    session.value = null;
    identity.value = null;
    prefs.sessionToken = "";
  }

  /** Server URL changed: drop the current session (it belongs to the old server). */
  function disconnect(): void {
    session.value?.close();
    session.value = null;
    identity.value = null;
  }

  function requireSession(): Session {
    if (!session.value || state.value !== "signed-in") throw RequestError.disconnected();
    return session.value;
  }

  return {
    session,
    state,
    identity,
    serverInfo,
    serverError,
    isSignedIn,
    playerName,
    loadServerInfo,
    resumeStoredSession,
    continueHere,
    signInAsGuest,
    signInWithGoogle,
    signOut,
    disconnect,
    requireSession,
  };
});
