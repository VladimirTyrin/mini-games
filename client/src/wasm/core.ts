import { fromBinary, toBinary } from "@bufbuild/protobuf";
import { BinaryReader } from "@bufbuild/protobuf/wire";

import {
  type BotKind,
  BotKindSchema,
  type GameCommand,
  GameCommandSchema,
  type GameSecretCommand,
  GameSecretCommandSchema,
  type GameSettings,
  GameSettingsSchema,
  type GameView,
  GameViewSchema,
} from "@/proto/minigames/v2/game_pb";
import {
  type MatchCatchUp,
  MatchCatchUpSchema,
  type MatchHeader,
  MatchHeaderSchema,
  type MatchResult,
  MatchResultSchema,
  type TickFrame,
  TickFrameSchema,
} from "@/proto/minigames/v2/match_pb";

import {
  type Refusal,
  RefusalSchema,
  type SettingProblem,
  SettingProblemSchema,
} from "@/proto/minigames/v2/refusal_pb";
import init, * as wasm from "./pkg/mg_wasm";
import wasmUrl from "./pkg/mg_wasm_bg.wasm?url";

/** What a command would lead to, asked of the rules before sending it. */
export type CommandPreview =
  | { readonly kind: "refused"; readonly refusal: Refusal }
  | { readonly kind: "continues" }
  | { readonly kind: "ends"; readonly result: MatchResult };

export interface GameInfo {
  id: string;
  name: string;
  minPlayers: number;
  maxPlayers: number;
  hasBots: boolean;
  cooperative: boolean;
  /** What a competitive match's scores measure: points of the game's own, or just who won. */
  scores: "points" | "winFlag";
}

let loading: Promise<void> | null = null;

/** Loads the WASM core once; every other function here requires it. */
export function loadCore(): Promise<void> {
  loading ??= init({ module_or_path: wasmUrl }).then(() => undefined);
  return loading;
}

export function gameCatalog(): GameInfo[] {
  return JSON.parse(wasm.gameCatalogJson()) as GameInfo[];
}

export function defaultSettings(gameId: string): GameSettings {
  return fromBinary(GameSettingsSchema, wasm.defaultSettings(gameId));
}

export function botKinds(gameId: string): BotKind[] {
  const reader = new BinaryReader(wasm.botKinds(gameId));
  const kinds: BotKind[] = [];
  while (reader.pos < reader.len) {
    kinds.push(fromBinary(BotKindSchema, reader.bytes()));
  }
  return kinds;
}

/** Bounds of one numeric setting, keyed by its wire field name (`field_width`, `win_count`...). */
export interface SettingRange {
  field: string;
  min: number;
  max: number;
}

/**
 * Why settings are invalid: `field` and `detail` the one setting at fault, if there is one
 * (`settingsProblemText` words it); else they do not suit the players seated.
 */
export interface SettingsProblem {
  field: string | null;
  detail: SettingProblem | null;
}

/** The game's own rules decide; returns the reason when `settings` are invalid. */
export function settingsProblem(settings: GameSettings, seats: number): SettingsProblem | null {
  const problem = wasm.validateSettings(toBinary(GameSettingsSchema, settings), seats);
  if (!problem) return null;
  try {
    const detail = problem.settingProblem;
    return { field: problem.field ?? null, detail: detail ? fromBinary(SettingProblemSchema, detail) : null };
  } finally {
    problem.free();
  }
}

/** The numeric bounds the game enforces for `settings`; some follow other fields. */
export function settingRanges(settings: GameSettings): SettingRange[] {
  return JSON.parse(wasm.settingRangesJson(toBinary(GameSettingsSchema, settings))) as SettingRange[];
}

/** Receiving side of lockstep. Call `free()` when done: WASM memory is not garbage-collected. */
export class Mirror {
  private readonly inner: wasm.MatchMirror;

  private constructor(inner: wasm.MatchMirror) {
    this.inner = inner;
  }

  static forHeader(header: MatchHeader): Mirror {
    return new Mirror(new wasm.MatchMirror(toBinary(MatchHeaderSchema, header)));
  }

  /** Validates a whole replay file (format, settings, frame log) and checks its end tick. */
  static replayEndTick(replayFile: Uint8Array): bigint {
    const mirror = wasm.MatchMirror.fromReplayFile(replayFile);
    try {
      return mirror.replayEndTick();
    } finally {
      mirror.free();
    }
  }

  /**
   * Replays the authority's full history (validated by the core: bounded gaps, increasing
   * ticks), so a hostile catch-up cannot make the page allocate millions of frames.
   */
  applyCatchUp(catchUp: MatchCatchUp): void {
    this.inner.applyCatchUp(toBinary(MatchCatchUpSchema, catchUp));
  }

  /** Throws on desync; the caller must rebuild from a full catch-up. */
  applyFrame(frame: TickFrame): void {
    this.inner.applyFrame(toBinary(TickFrameSchema, frame));
  }

  view(slot: number | null): GameView {
    return fromBinary(GameViewSchema, this.inner.view(slot ?? -1));
  }

  get nextTick(): number {
    return this.inner.nextTick();
  }

  /** Milliseconds per frame from the game's own clock; 0 for turn-based games. */
  get tickMs(): number {
    return this.inner.tickMs();
  }

  get isFinished(): boolean {
    return this.inner.isFinished();
  }

  result(): MatchResult | null {
    const bytes = this.inner.result();
    return bytes.length === 0 && !this.inner.isFinished() ? null : fromBinary(MatchResultSchema, bytes);
  }

  stateHash(): string {
    return this.inner.stateHash();
  }

  /** Why the rules would refuse `command` from `slot` now (`refusalText` words it); `null` if they accept it. */
  refusalOf(slot: number, command: GameCommand): Refusal | null {
    const refusal = this.inner.refusalOf(slot, toBinary(GameCommandSchema, command));
    return refusal ? fromBinary(RefusalSchema, refusal) : null;
  }

  /** Why the rules would refuse the private `command` from `slot`; `null` if they accept it. */
  secretRefusalOf(slot: number, command: GameSecretCommand): Refusal | null {
    const refusal = this.inner.secretRefusalOf(slot, toBinary(GameSecretCommandSchema, command));
    return refusal ? fromBinary(RefusalSchema, refusal) : null;
  }

  /** The private command the game would choose for `slot` (a random fleet), if it has one. */
  defaultSecretCommand(slot: number): GameSecretCommand | null {
    const seed = crypto.getRandomValues(new Uint32Array(2));
    const bytes = this.inner.defaultSecretCommand(slot, seed[0] ?? 0, seed[1] ?? 0);
    return bytes.length === 0 ? null : fromBinary(GameSecretCommandSchema, bytes);
  }

  /** What `command` from `slot` would lead to: refused, the game goes on, or how it ends. */
  preview(slot: number, command: GameCommand): CommandPreview {
    const encoded = toBinary(GameCommandSchema, command);
    const refusal = this.inner.refusalOf(slot, encoded);
    if (refusal !== undefined) return { kind: "refused", refusal: fromBinary(RefusalSchema, refusal) };
    const result = this.inner.resultAfter(slot, encoded);
    return result.length === 0
      ? { kind: "continues" }
      : { kind: "ends", result: fromBinary(MatchResultSchema, result) };
  }

  free(): void {
    this.inner.free();
  }
}
