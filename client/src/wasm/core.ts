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

import init, * as wasm from "./pkg/mg_wasm";
import wasmUrl from "./pkg/mg_wasm_bg.wasm?url";

/** What a command would lead to, asked of the rules before sending it. */
export type CommandPreview =
  | { readonly kind: "refused"; readonly reason: string }
  | { readonly kind: "continues" }
  | { readonly kind: "ends"; readonly result: MatchResult };

export interface GameInfo {
  id: string;
  name: string;
  minPlayers: number;
  maxPlayers: number;
  hasBots: boolean;
  cooperative: boolean;
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

/** Why settings are invalid; `field` names the one setting at fault, if there is one. */
export interface SettingsProblem {
  field: string | null;
  message: string;
}

/** The game's own rules decide; returns the reason when `settings` are invalid. */
export function settingsProblem(settings: GameSettings, seats: number): SettingsProblem | null {
  const problem = wasm.validateSettings(toBinary(GameSettingsSchema, settings), seats);
  if (!problem) return null;
  try {
    return { field: problem.field ?? null, message: problem.message };
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

  /** Why the rules would refuse `command` from `slot` now, for the player; `null` if they accept it. */
  refusalOf(slot: number, command: GameCommand): string | null {
    return this.inner.refusalOf(slot, toBinary(GameCommandSchema, command)) ?? null;
  }

  /** Why the rules would refuse the private `command` from `slot`; `null` if they accept it. */
  secretRefusalOf(slot: number, command: GameSecretCommand): string | null {
    return this.inner.secretRefusalOf(slot, toBinary(GameSecretCommandSchema, command)) ?? null;
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
    const reason = this.inner.refusalOf(slot, encoded);
    if (reason !== undefined) return { kind: "refused", reason };
    const result = this.inner.resultAfter(slot, encoded);
    return result.length === 0
      ? { kind: "continues" }
      : { kind: "ends", result: fromBinary(MatchResultSchema, result) };
  }

  free(): void {
    this.inner.free();
  }
}
