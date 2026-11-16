/** How the session signs in; tried again on every reconnect. */
export type Credentials =
  | { kind: "guest"; displayName: string }
  | { kind: "google"; idToken: string }
  | { kind: "session"; token: string };
