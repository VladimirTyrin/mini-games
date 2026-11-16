// Push notifications: FCM in the Android app, Web Push in browsers and home-screen web apps
// (the service worker side is public/push-sw.js).
import type { MessageInitShape } from "@bufbuild/protobuf";

import type { PushTargetSchema } from "@/proto/minigames/v2/push_pb";

import { isAppleMobile, isInstalledWebApp } from "./web-app-install";

export type PushTargetInit = MessageInitShape<typeof PushTargetSchema>;

/** Whether this device can get notifications, and if not, what the player can do about it. */
export type PushAvailability = "available" | "add-to-home-screen-first" | "unsupported" | "blocked";

/** A notification the player opened (or that arrived while the app was open). */
export interface ReceivedNotification {
  kind: string;
  /** Server clock, unix ms. */
  sentAt: number;
  /** Device clock: when the device showed it, if anything saw that moment. */
  receivedAt: number | null;
  /** When the player tapped it; null when it arrived while the app was in front. */
  tappedAt: number | null;
  /** When the app had it in hand: after the tap, it was ready to show it. */
  handledAt: number;
  /** Everything the server sent along, as strings (`inviteId`, `lobbyId`...). */
  data: Record<string, string>;
}

/** Both the Android channel and the server's FCM messages use this id. */
const CHANNEL_ID = "invites";
/** Google Play services or the service worker normally answer in well under a second. */
const REGISTRATION_TIMEOUT_MS = 15_000;

export function pushAvailability(): PushAvailability {
  if (__NATIVE_APP__) return "available";
  const supported = "serviceWorker" in navigator && "PushManager" in window && "Notification" in window;
  // Safari on iOS offers push only to web apps started from the home screen.
  if (isAppleMobile() && !isInstalledWebApp()) return "add-to-home-screen-first";
  if (!supported) return "unsupported";
  return Notification.permission === "denied" ? "blocked" : "available";
}

/** Notifications are already allowed: the device can register without a prompt. */
export async function canNotifyWithoutAsking(): Promise<boolean> {
  if (__NATIVE_APP__) {
    const { PushNotifications } = await import("@capacitor/push-notifications");
    return (await PushNotifications.checkPermissions()).receive === "granted";
  }
  return pushAvailability() === "available" && Notification.permission === "granted";
}

/**
 * Asks for permission (the first await: iOS lets only a direct tap ask) and returns where this
 * device receives pushes. `webPushPublicKey` is the server's (`ServerInfo`).
 */
export async function registerForPush(webPushPublicKey: string): Promise<PushTargetInit> {
  if (__NATIVE_APP__) return { target: { case: "fcmToken", value: await nativeToken() } };
  return { target: { case: "webPush", value: await webSubscription(webPushPublicKey) } };
}

async function nativeToken(): Promise<string> {
  const { PushNotifications } = await import("@capacitor/push-notifications");
  let permission = await PushNotifications.checkPermissions();
  if (permission.receive !== "granted") permission = await PushNotifications.requestPermissions();
  if (permission.receive !== "granted") throw new Error("Notifications are turned off for this app");
  // High importance: the notification pops up over other apps instead of only joining the shade.
  await PushNotifications.createChannel({
    id: CHANNEL_ID,
    name: "Game invites",
    importance: 5,
    visibility: 1,
    vibration: true,
  });
  const listeners: Promise<{ remove: () => Promise<void> }>[] = [];
  try {
    return await withTimeout(
      new Promise<string>((resolve, reject) => {
        listeners.push(PushNotifications.addListener("registration", (token) => resolve(token.value)));
        listeners.push(
          PushNotifications.addListener("registrationError", (error) => reject(new Error(error.error))),
        );
        PushNotifications.register().catch(reject);
      }),
      "Google Play services did not answer",
    );
  } finally {
    for (const listener of listeners) void listener.then((handle) => handle.remove());
  }
}

async function webSubscription(publicKey: string) {
  const permission = await Notification.requestPermission();
  if (permission !== "granted") throw new Error("Notifications are not allowed for this site");
  if (!publicKey) throw new Error("This server does not send notifications");
  const registration = await withTimeout(navigator.serviceWorker.ready, "The offline worker is not running");
  const serverKey = fromBase64Url(publicKey);
  let subscription = await registration.pushManager.getSubscription();
  // A subscription made for another server key (the server's key was replaced) is useless.
  if (subscription && !sameBytes(subscription.options.applicationServerKey, serverKey)) {
    await subscription.unsubscribe();
    subscription = null;
  }
  subscription ??= await registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: serverKey,
  });
  const { endpoint, keys } = subscription.toJSON();
  if (!endpoint || !keys?.p256dh || !keys.auth)
    throw new Error("The browser returned an incomplete subscription");
  return { endpoint, p256dh: keys.p256dh, auth: keys.auth };
}

/** Message the service worker posts to an open window when its notification is tapped. */
interface NotificationOpenedMessage {
  type: "notification-opened";
  data: Record<string, string>;
}

/**
 * Calls `handler` for every notification the player opens, including the one that started the
 * app, and (in the Android app) for one that arrives while the app is in front.
 */
export async function onNotification(handler: (notification: ReceivedNotification) => void): Promise<void> {
  if (__NATIVE_APP__) {
    const { PushNotifications } = await import("@capacitor/push-notifications");
    // The plugin keeps the tap that launched the app until this listener exists.
    await PushNotifications.addListener("pushNotificationActionPerformed", (action) => {
      const notification = parseNotification(action.notification.data, Date.now());
      if (notification) handler(notification);
    });
    await PushNotifications.addListener("pushNotificationReceived", (received) => {
      const now = Date.now();
      const notification = parseNotification({ ...received.data, receivedAt: String(now) }, now);
      if (notification) handler({ ...notification, tappedAt: null });
    });
    return;
  }
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.addEventListener("message", (event: MessageEvent<NotificationOpenedMessage>) => {
      if (event.data?.type !== "notification-opened") return;
      const notification = parseNotification(event.data.data, Date.now());
      if (notification) handler(notification);
    });
  }
  // A tap with no window open starts the app with the notification in the address.
  const url = new URL(location.href);
  const opened = url.searchParams.get("notification");
  if (opened === null) return;
  url.searchParams.delete("notification");
  history.replaceState(history.state, "", url.href);
  try {
    const notification = parseNotification(JSON.parse(opened) as Record<string, string>, Date.now());
    if (notification) handler(notification);
  } catch {
    // Not ours, or mangled: ignore it like any other unknown parameter.
  }
}

function parseNotification(data: Record<string, unknown>, handledAt: number): ReceivedNotification | null {
  const number = (key: string): number | null => {
    const value = Number(data[key]);
    return Number.isFinite(value) && value > 0 ? value : null;
  };
  const sentAt = number("sentAt");
  if (typeof data.kind !== "string" || sentAt === null) return null;
  const strings = Object.fromEntries(
    Object.entries(data).filter((entry): entry is [string, string] => typeof entry[1] === "string"),
  );
  return {
    kind: data.kind,
    sentAt,
    receivedAt: number("receivedAt"),
    tappedAt: number("tappedAt"),
    handledAt,
    data: strings,
  };
}

function withTimeout<T>(promise: Promise<T>, message: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), REGISTRATION_TIMEOUT_MS);
    promise.then(resolve, reject).finally(() => clearTimeout(timer));
  });
}

function fromBase64Url(text: string): Uint8Array<ArrayBuffer> {
  const base64 = text.replaceAll("-", "+").replaceAll("_", "/");
  const binary = atob(base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "="));
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

function sameBytes(key: ArrayBuffer | null, expected: Uint8Array): boolean {
  if (!key || key.byteLength !== expected.length) return false;
  const actual = new Uint8Array(key);
  return actual.every((byte, index) => byte === expected[index]);
}
