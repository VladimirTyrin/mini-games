/** Messages between the page and the local-authority worker (binary protobuf payloads). */
export type ToWorker =
  | {
      type: "start";
      header: Uint8Array;
      seedHigh: number;
      seedLow: number;
      /** Seats whose secrets a spectator is shown; 0 when the person here plays. */
      watchedSeats: number;
    }
  | { type: "submit"; requestId: number; slot: number; command: Uint8Array }
  | { type: "submitSecret"; requestId: number; slot: number; command: Uint8Array }
  | { type: "pause" }
  | { type: "resume" }
  | { type: "replay"; requestId: number }
  | { type: "stop" };

export type FromWorker =
  | { type: "frame"; frame: Uint8Array }
  | { type: "accepted"; requestId: number }
  /** Encoded `GameSeatSecret` of the player's seat. */
  | { type: "seatSecret"; secret: Uint8Array }
  /** Encoded `GameSeatSecret` of `slot`, for a spectator (games that show them). */
  | { type: "spectatorSecret"; slot: number; secret: Uint8Array }
  | { type: "rejected"; requestId: number; message: string }
  | { type: "replay"; requestId: number; replay: Uint8Array | null }
  | { type: "ended" }
  | { type: "failed"; message: string };
