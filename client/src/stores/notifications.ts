import { defineStore } from "pinia";
import { ref } from "vue";
import type { Router } from "vue-router";

import { type ReceivedNotification, onNotification, registerForPush } from "@/platform/push-notifications";
import { errorMessage } from "@/util/error-message";

import { useConnectionStore } from "./connection";
import { useFriendsStore } from "./friends";

/** Time to close the app or lock the phone before the test notification is sent. */
export const TEST_PUSH_DELAY_SECONDS = 20;

export type TestPushStatus =
  | { kind: "idle" }
  | { kind: "registering" }
  /** Device clock time the server sends it at. */
  | { kind: "scheduled"; sendsAt: number }
  | { kind: "failed"; message: string };

export const useNotificationsStore = defineStore("notifications", () => {
  const connection = useConnectionStore();
  const testStatus = ref<TestPushStatus>({ kind: "idle" });
  /** The last test notification seen, with the moments that tell how long each step took. */
  const lastTest = ref<ReceivedNotification | null>(null);

  /** What a tap on each kind of notification opens. */
  async function listen(router: Router): Promise<void> {
    const friends = useFriendsStore();
    await onNotification((notification) => {
      const tapped = notification.tappedAt !== null;
      switch (notification.kind) {
        case "test":
          lastTest.value = notification;
          testStatus.value = { kind: "idle" };
          if (tapped) void router.push("/settings");
          break;
        case "invite": {
          // One that arrives while the app is open is shown by the app itself (GameInvite).
          const { inviteId, lobbyId } = notification.data;
          if (tapped && inviteId && lobbyId) void friends.openInvite(inviteId, lobbyId);
          break;
        }
        case "friends":
          if (tapped) void router.push("/friends");
          break;
      }
    });
  }

  /** Call straight from a tap: the permission prompt must come first (iOS). */
  async function sendTest(): Promise<void> {
    testStatus.value = { kind: "registering" };
    try {
      const target = await registerForPush(connection.serverInfo?.webPushPublicKey ?? "");
      await connection.requireSession().request({
        case: "requestTestPush",
        value: { target, delaySeconds: TEST_PUSH_DELAY_SECONDS },
      });
      testStatus.value = { kind: "scheduled", sendsAt: Date.now() + TEST_PUSH_DELAY_SECONDS * 1000 };
    } catch (error) {
      testStatus.value = { kind: "failed", message: errorMessage(error) };
    }
  }

  return { testStatus, lastTest, listen, sendTest };
});
