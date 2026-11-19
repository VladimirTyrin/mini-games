import { onBeforeUnmount, ref } from "vue";

import { t } from "@/i18n";
import { InviteState, type InviteUpdate } from "@/proto/minigames/v2/social_pb";
import { useFriendsStore } from "@/stores/friends";

/** What became of this device's invitations from the current lobby, counted down every second. */
export function useInviteStatus() {
  const friends = useFriendsStore();
  const now = ref(Date.now());
  const ticker = setInterval(() => (now.value = Date.now()), 1000);
  onBeforeUnmount(() => clearInterval(ticker));

  /** The latest invitation this device sent to that account, if any. */
  function invitationTo(accountId: string): InviteUpdate | undefined {
    let latest = undefined;
    for (const update of friends.sentInvites.values()) {
      if (update.friend?.id === accountId) latest = update;
    }
    return latest;
  }

  /**
   * Pending until its time is up, even if the news of its end never arrived; accepted only for a
   * while, since a friend who accepted and never came is free to be asked again.
   */
  function stateOf(accountId: string): InviteState | undefined {
    const update = invitationTo(accountId);
    const timeUp = update !== undefined && friends.deadlineOf(update.inviteId) <= now.value;
    if (update?.state === InviteState.PENDING && timeUp) return InviteState.EXPIRED;
    if (update?.state === InviteState.ACCEPTED && timeUp) return undefined;
    return update?.state;
  }

  function status(accountId: string): string | null {
    const update = invitationTo(accountId);
    switch (stateOf(accountId)) {
      case InviteState.PENDING: {
        const seconds = Math.max(
          0,
          Math.floor((friends.deadlineOf(update?.inviteId ?? "") - now.value) / 1000),
        );
        return t.friends.invite.waiting(seconds);
      }
      case InviteState.ACCEPTED:
        return t.friends.invite.coming;
      case InviteState.DECLINED:
        return t.friends.invite.declined;
      case InviteState.EXPIRED:
        return t.friends.invite.noAnswer;
      default:
        return null;
    }
  }

  function isPending(accountId: string): boolean {
    return stateOf(accountId) === InviteState.PENDING;
  }

  function cancel(accountId: string): void {
    const update = invitationTo(accountId);
    if (update) void friends.cancelInvite(update.inviteId);
  }

  return { status, isPending, cancel };
}
