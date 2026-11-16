/** Messages between the page and the local-authority worker (binary protobuf payloads). */
export type ToWorker =
  | { type: "start"; header: Uint8Array; seedHigh: number; seedLow: number }
  | { type: "submit"; requestId: number; slot: number; command: Uint8Array }
  | { type: "pause" }
  | { type: "resume" }
  | { type: "replay"; requestId: number }
  | { type: "stop" };

export type FromWorker =
  | { type: "frame"; frame: Uint8Array }
  | { type: "accepted"; requestId: number }
  | { type: "rejected"; requestId: number; message: string }
  | { type: "replay"; requestId: number; replay: Uint8Array }
  | { type: "ended" }
  | { type: "failed"; message: string };
