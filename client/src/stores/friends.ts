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
} from "@/proto/minigames/v2/social_pb";
import { canNotifyWithoutAsking, registerForPush } from "@/platform/push-notifications";

import { useConnectionStore } from "./connection";
import { useLobbyStore } from "./lobby";
import { useToastStore } from "./toasts";

/** A friend is an account; a player in a lobby is one of its devices (`<account>.<device>`). */
export function accountOf(playerId: string): string {
  return playerId.split(".")[0] ?? playerId;
}

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
  /** What became of the invitations this device sent, by invite id. */
  const sentInvites = ref<Map<string, InviteUpdate>>(new Map());
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
          incomingInvites.value = [
            ...incomingInvites.value.filter((open) => open.inviteId !== invite.inviteId),
            invite,
          ];
        }),
        session.events.on("gameInviteClosed", ({ inviteId }) => forgetInvite(inviteId)),
        session.events.on("inviteUpdate", (update) => {
          sentInvites.value = new Map(sentInvites.value).set(update.inviteId, update);
          const name = update.friend?.displayName ?? "Your friend";
          if (update.state === InviteState.DECLINED) toasts.show(`${name} declined the invitation`);
          if (update.state === InviteState.EXPIRED) toasts.show(`${name} did not answer`);
        }),
        session.events.on("signedIn", () => void signedIn()),
      );
    },
    { immediate: true },
  );

  async function signedIn(): Promise<void> {
    if (!hasAccount.value) return;
    await refresh();
    void registerPush(false);
    if (pendingLinkCode.value) await useLink(pendingLinkCode.value);
    if (openedInvite.value) await acceptOpenedInvite();
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
    pendingLinkCode.value = null;
    try {
      await connection.requireSession().request({ case: "useFriendLink", value: { code } });
      toasts.show("You have a new friend", "success");
    } catch (error) {
      toasts.failure(error);
    }
  }

  const sendRequest = (playerId: string) =>
    call({ case: "sendFriendRequest", value: { playerId } }).then(() =>
      toasts.show("Friend request sent", "success"),
    );
  const answerRequest = (accountId: string, accept: boolean) =>
    call({
      case: "answerFriendRequest",
      value: {
        accountId,
        answer: accept ? FriendRequestAnswer.ACCEPT : FriendRequestAnswer.DECLINE,
      },
    });
  const remove = (accountId: string) => call({ case: "removeFriend", value: { accountId } });
  const invite = (accountId: string) => call({ case: "inviteFriend", value: { accountId } });
  const cancelInvite = (inviteId: string) => call({ case: "cancelInvite", value: { inviteId } });

  /** Accepts and joins the lobby; leaves the current lobby first. */
  async function accept(inviteId: string, lobbyId: string): Promise<void> {
    forgetInvite(inviteId);
    try {
      await connection
        .requireSession()
        .request({ case: "answerInvite", value: { inviteId, answer: InviteAnswer.ACCEPT } });
    } catch (error) {
      if (error instanceof RequestError && error.code === ErrorCode.INVITE_CLOSED) {
        toasts.show("That invitation is no longer open");
      } else {
        toasts.failure(error);
      }
      return;
    }
    if (lobby.current?.lobbyId === lobbyId) return;
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
    refresh,
    registerPush,
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
