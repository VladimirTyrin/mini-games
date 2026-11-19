import { defineStore } from "pinia";
import { computed, ref, shallowRef, watch } from "vue";

import { RequestError } from "@/net/request-error";
import type { ClientPayload, ServerPayload } from "@/net/session";
import { IdentityKind } from "@/proto/minigames/v2/identity_pb";
import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";
import type { FriendLinkKind } from "@/proto/minigames/v2/social_pb";
import {
  type FriendLink,
  type FriendList,
  FriendRequestAnswer,
  type GameInvite,
  InviteAnswer,
  InviteState,
  type InviteUpdate,
  Presence,
} from "@/proto/minigames/v2/social_pb";
import { t } from "@/i18n";
import { canNotifyWithoutAsking, pushAvailability, registerForPush } from "@/platform/push-notifications";
import { storage } from "@/platform/storage";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";
import { useMatchStore } from "./match";
import { useToastStore } from "./toasts";

/** A friend is an account; a player in a lobby is one of its devices (`<account>.<device>`). */
export function accountOf(playerId: string): string {
  return playerId.split(".")[0] ?? playerId;
}

/** Set once the app asked for notifications on its own: asking again after a "no" only annoys. */
const ASKED_FOR_NOTIFICATIONS_KEY = "mini-games.notifications-asked";

/** How long "Coming!" shows for a friend who accepted but has not come into the lobby yet. */
const ACCEPTED_SHOWN_MS = 30_000;

/** An invitation the player opened from a notification, kept until the app is signed in. */
interface OpenedInvite {
  inviteId: string;
  lobbyId: string;
}

export const useFriendsStore = defineStore("friends", () => {
  const connection = useConnectionStore();
  const lobby = useLobbyStore();
  const toasts = useToastStore();

  const list = shallowRef<FriendList | null>(null);
  /** Invitations to this account still open, newest last. */
  const incomingInvites = ref<GameInvite[]>([]);
  /** What became of the invitations this device sent from the current lobby, by invite id. */
  const sentInvites = ref<Map<string, InviteUpdate>>(new Map());
  /**
   * When each invitation runs out, on this device's clock: counted from the time left the server
   * sent, since the device's clock may be off.
   */
  const deadlines = ref<Map<string, number>>(new Map());
  /** A friend link the app was opened with, used once signed in with Google. */
  const pendingLinkCode = ref<string | null>(null);
  const openedInvite = ref<OpenedInvite | null>(null);

  const hasAccount = computed(
    () => connection.isSignedIn && connection.identity?.kind === IdentityKind.GOOGLE,
  );
  const friendAccounts = computed(
    () => new Set(list.value?.friends.map((friend) => friend.identity?.id ?? "") ?? []),
  );
  const requestedAccounts = computed(
    () =>
      new Set([
        ...(list.value?.outgoingRequests.map((identity) => identity.id) ?? []),
        ...(list.value?.incomingRequests.map((identity) => identity.id) ?? []),
      ]),
  );
  const incomingRequestCount = computed(() => list.value?.incomingRequests.length ?? 0);
  /** Playing first, then online, then the rest; by name within each. */
  const sortedFriends = computed(() => {
    const rank: Partial<Record<Presence, number>> = { [Presence.PLAYING]: 0, [Presence.ONLINE]: 1 };
    return [...(list.value?.friends ?? [])].sort(
      (a, b) =>
        (rank[a.presence] ?? 2) - (rank[b.presence] ?? 2) ||
        (a.identity?.displayName ?? "").localeCompare(b.identity?.displayName ?? ""),
    );
  });

  let detach: (() => void)[] = [];
  watch(
    () => connection.session,
    (session) => {
      for (const off of detach) off();
      detach = [];
      list.value = null;
      incomingInvites.value = [];
      sentInvites.value = new Map();
      if (!session) return;
      detach.push(
        session.events.on("friendList", (friends) => (list.value = friends)),
        session.events.on("friendPresence", ({ accountId, presence }) => {
          if (!list.value) return;
          const friend = list.value.friends.find((entry) => entry.identity?.id === accountId);
          if (!friend) return;
          friend.presence = presence;
          list.value = { ...list.value };
        }),
        session.events.on("gameInvite", (invite) => {
          setDeadline(invite.inviteId, invite.expiresInMs);
          incomingInvites.value = [
            ...incomingInvites.value.filter((open) => open.inviteId !== invite.inviteId),
            invite,
          ];
        }),
        session.events.on("gameInviteClosed", ({ inviteId }) => forgetInvite(inviteId)),
        session.events.on("inviteUpdate", (update) => {
          if (update.state === InviteState.PENDING) setDeadline(update.inviteId, update.expiresInMs);
          // "Coming!" until the friend shows up in the lobby, or for a while if they never do.
          if (update.state === InviteState.ACCEPTED) setDeadline(update.inviteId, ACCEPTED_SHOWN_MS);
          sentInvites.value = new Map(sentInvites.value).set(update.inviteId, update);
          const name = update.friend?.displayName ?? t.friends.yourFriend;
          if (update.state === InviteState.DECLINED) toasts.show(t.friends.declinedInvitation(name));
          if (update.state === InviteState.EXPIRED) toasts.show(t.friends.didNotAnswer(name));
        }),
        session.events.on("signedIn", () => void signedIn()),
      );
      // This store is made at start-up, after the app began signing in: the first sign-in may
      // be over already, and its event gone.
      if (connection.isSignedIn) void signedIn();
    },
    { immediate: true },
  );
  // What happened to invitations into an earlier lobby says nothing about this one.
  watch(
    () => lobby.current?.lobbyId,
    () => (sentInvites.value = new Map()),
  );
  // A friend who came in needs no news of the invitation any more; once gone again, they can be
  // invited afresh.
  watch(
    () => lobby.members.map((member) => (member ? accountOf(member.id) : "")),
    (accounts) => {
      const present = new Set(accounts);
      const kept = [...sentInvites.value].filter(([, update]) => !present.has(update.friend?.id ?? ""));
      if (kept.length !== sentInvites.value.size) sentInvites.value = new Map(kept);
    },
  );

  function setDeadline(inviteId: string, expiresInMs: number): void {
    deadlines.value = new Map(deadlines.value).set(inviteId, Date.now() + expiresInMs);
  }

  function deadlineOf(inviteId: string): number {
    return deadlines.value.get(inviteId) ?? 0;
  }

  async function signedIn(): Promise<void> {
    if (!hasAccount.value) return;
    await refresh();
    // Before answering an opened invitation: the server then knows this device answered it and
    // does not notify it that "another device" did.
    await registerPush(false);
    if (pendingLinkCode.value) await useLink(pendingLinkCode.value);
    if (openedInvite.value) await acceptOpenedInvite();
  }

  /**
   * Asks for notifications the first time the player does something with friends: from a tap (iOS
   * lets nothing else ask), and only once, whatever the answer.
   */
  function askForNotificationsOnce(): void {
    if (!hasAccount.value || storage.get(ASKED_FOR_NOTIFICATIONS_KEY) !== null) return;
    if (!__NATIVE_APP__ && (pushAvailability() !== "available" || Notification.permission !== "default")) {
      return;
    }
    storage.set(ASKED_FOR_NOTIFICATIONS_KEY, "1");
    void registerPush(true);
  }

  function forgetInvite(inviteId: string): void {
    incomingInvites.value = incomingInvites.value.filter((invite) => invite.inviteId !== inviteId);
  }

  async function call(message: ClientPayload): Promise<ServerPayload> {
    try {
      return await connection.requireSession().request(message);
    } catch (error) {
      toasts.failure(error);
      throw error;
    }
  }

  async function refresh(): Promise<void> {
    const reply = await call({ case: "listFriends", value: {} }).catch(() => null);
    if (reply?.case === "friendList") list.value = reply.value;
  }

  /**
   * Tells the server where this device gets notifications. `ask`: may show the permission prompt
   * (only from a tap); otherwise only when notifications are already allowed.
   */
  async function registerPush(ask: boolean): Promise<boolean> {
    if (!hasAccount.value) return false;
    if (!ask && !(await canNotifyWithoutAsking())) return false;
    try {
      const target = await registerForPush(connection.serverInfo?.webPushPublicKey ?? "");
      await connection.requireSession().request({ case: "registerPushTarget", value: { target } });
      return true;
    } catch (error) {
      if (ask) toasts.failure(error);
      return false;
    }
  }

  async function createLink(kind: FriendLinkKind): Promise<FriendLink | null> {
    const reply = await call({ case: "createFriendLink", value: { kind } }).catch(() => null);
    return reply?.case === "friendLink" ? reply.value : null;
  }

  const revokeLink = (code: string) => call({ case: "revokeFriendLink", value: { code } });

  /** Becomes friends with the link's owner; kept for later when not signed in with Google yet. */
  async function useLink(code: string): Promise<void> {
    if (!hasAccount.value) {
      pendingLinkCode.value = code;
      return;
    }
    askForNotificationsOnce();
    pendingLinkCode.value = null;
    try {
      await connection.requireSession().request({ case: "useFriendLink", value: { code } });
      toasts.show(t.friends.newFriend, "success");
    } catch (error) {
      toasts.failure(error);
    }
  }

  function sendRequest(playerId: string): Promise<void> {
    askForNotificationsOnce();
    return call({ case: "sendFriendRequest", value: { playerId } }).then(() =>
      toasts.show(t.friends.requestSent, "success"),
    );
  }
  function answerRequest(accountId: string, accept: boolean): Promise<ServerPayload> {
    if (accept) askForNotificationsOnce();
    return call({
      case: "answerFriendRequest",
      value: {
        accountId,
        answer: accept ? FriendRequestAnswer.ACCEPT : FriendRequestAnswer.DECLINE,
      },
    });
  }
  const remove = (accountId: string) => call({ case: "removeFriend", value: { accountId } });
  function invite(accountId: string): Promise<ServerPayload> {
    askForNotificationsOnce();
    return call({ case: "inviteFriend", value: { accountId } });
  }

  async function cancelInvite(inviteId: string): Promise<void> {
    try {
      await connection.requireSession().request({ case: "cancelInvite", value: { inviteId } });
    } catch (error) {
      if (!(error instanceof RequestError && error.code === ErrorCode.INVITE_CLOSED)) {
        toasts.failure(error);
        return;
      }
      // It closed already and the news did not reach this device: stop waiting for it.
      const update = sentInvites.value.get(inviteId);
      if (update) {
        sentInvites.value = new Map(sentInvites.value).set(inviteId, {
          ...update,
          state: InviteState.EXPIRED,
        });
      }
    }
  }

  /** Accepts and joins the lobby; leaves the current lobby first. */
  async function accept(inviteId: string, lobbyId: string): Promise<void> {
    forgetInvite(inviteId);
    try {
      await connection
        .requireSession()
        .request({ case: "answerInvite", value: { inviteId, answer: InviteAnswer.ACCEPT } });
    } catch (error) {
      if (error instanceof RequestError && error.code === ErrorCode.INVITE_CLOSED) {
        toasts.show(t.friends.invitationClosed);
      } else {
        toasts.failure(error);
      }
      return;
    }
    if (lobby.current?.lobbyId === lobbyId) return;
    // The match on screen goes with the lobby; left on screen it would freeze.
    const match = useMatchStore();
    if (match.isInOnlineMatch) match.close();
    if (lobby.current) await lobby.leave();
    await lobby.join(lobbyId, false).catch(() => undefined);
  }

  async function decline(inviteId: string): Promise<void> {
    forgetInvite(inviteId);
    await connection
      .requireSession()
      .request({ case: "answerInvite", value: { inviteId, answer: InviteAnswer.DECLINE } })
      .catch(() => undefined);
  }

  /** A tap on an invitation notification: join once signed in. */
  async function openInvite(inviteId: string, lobbyId: string): Promise<void> {
    openedInvite.value = { inviteId, lobbyId };
    if (hasAccount.value) await acceptOpenedInvite();
  }

  async function acceptOpenedInvite(): Promise<void> {
    const opened = openedInvite.value;
    openedInvite.value = null;
    if (opened) await accept(opened.inviteId, opened.lobbyId);
  }

  return {
    list,
    incomingInvites,
    sentInvites,
    pendingLinkCode,
    hasAccount,
    friendAccounts,
    requestedAccounts,
    incomingRequestCount,
    sortedFriends,
    refresh,
    registerPush,
    askForNotificationsOnce,
    deadlineOf,
    createLink,
    revokeLink,
    useLink,
    sendRequest,
    answerRequest,
    remove,
    invite,
    cancelInvite,
    accept,
    decline,
    openInvite,
  };
});
