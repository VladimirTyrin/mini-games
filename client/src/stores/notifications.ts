import { defineStore } from "pinia";
import { ref } from "vue";
import type { Router } from "vue-router";

import { type ReceivedNotification, onNotification, registerForPush } from "@/platform/push-notifications";
import { errorMessage } from "@/util/error-message";

import { useConnectionStore } from "./connection";

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

  async function listen(router: Router): Promise<void> {
    await onNotification((notification) => {
      if (notification.kind !== "test") return;
      lastTest.value = notification;
      testStatus.value = { kind: "idle" };
      if (notification.tappedAt !== null) void router.push("/settings");
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
