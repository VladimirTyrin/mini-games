import type { MessageInitShape } from "@bufbuild/protobuf";

import type { GameCommandSchema } from "@/proto/minigames/v2/game_pb";
import type {
  MatchCatchUp,
  MatchHeader,
  MatchPause,
  MatchResult,
  TickFrame,
} from "@/proto/minigames/v2/match_pb";

export type GameCommandInit = MessageInitShape<typeof GameCommandSchema>;

export interface MatchSourceEvents {
  frame: TickFrame;
  /** Full history; replaces everything applied so far. */
  catchUp: MatchCatchUp;
  /** Every seat the match now waits for; replaces the previous list. */
  paused: MatchPause[];
  resumed: { countdownMs: number };
  /** `null`: the authority aborted the match (no result, no replay). */
  ended: MatchResult | null;
}

export type MatchSourceKind =
  /** Server authority. */
  | "online"
  /** In-browser authority (no other humans). */
  | "local"
  /** Pre-recorded frames. */
  | "replay";

/**
 * Where lockstep frames come from. The match screen only talks to this interface, so online,
 * offline and replay playback share one code path.
 */
export interface MatchSource {
  readonly kind: MatchSourceKind;
  readonly header: MatchHeader;
  /** Seat of this client; `null` for observers and replays. */
  readonly mySlot: number | null;
  on<K extends keyof MatchSourceEvents>(
    event: K,
    handler: (payload: MatchSourceEvents[K]) => void,
  ): () => void;
  /** Starts delivering events; call after subscribing. */
  start(): void;
  /** Rejects with a user-facing message when the authority refuses the command. */
  submit(command: GameCommandInit): Promise<void>;
  requestCatchUp(): void;
  voteDrop(slot: number): Promise<void>;
  leave(): void;
  /** Encoded ReplayFile, when available. */
  replayFile(): Promise<Uint8Array | null>;
}
