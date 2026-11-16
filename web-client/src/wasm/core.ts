import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { BinaryReader } from "@bufbuild/protobuf/wire";

import {
  type BotKind,
  BotKindSchema,
  type GameSettings,
  GameSettingsSchema,
  type GameView,
  GameViewSchema,
} from "@/proto/minigames/v2/game_pb";
import {
  type MatchHeader,
  MatchHeaderSchema,
  type MatchResult,
  MatchResultSchema,
  type TickFrame,
  TickFrameSchema,
} from "@/proto/minigames/v2/match_pb";

import init, * as wasm from "./pkg/mg_wasm";
import wasmUrl from "./pkg/mg_wasm_bg.wasm?url";

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

/** The game's own rules decide; returns the reason when `settings` are invalid. */
export function settingsProblem(settings: GameSettings, seats: number): string | null {
  return wasm.validateSettings(toBinary(GameSettingsSchema, settings), seats) ?? null;
}

/** Receiving side of lockstep. Call `free()` when done: WASM memory is not garbage-collected. */
export class Mirror {
  private readonly inner: wasm.MatchMirror;

  constructor(header: MatchHeader) {
    this.inner = new wasm.MatchMirror(toBinary(MatchHeaderSchema, header));
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

  free(): void {
    this.inner.free();
  }
}

export function emptyFrame(tick: bigint): TickFrame {
  return create(TickFrameSchema, { tick, inputs: [] });
}

/** Restores a gap-free frame sequence from a compact log that omits empty frames. */
export function* expandCompactLog(
  frames: readonly TickFrame[],
  from: bigint,
  end: bigint,
): Generator<TickFrame> {
  let tick = from;
  for (const frame of frames) {
    if (frame.tick < tick) continue;
    while (tick < frame.tick && tick < end) {
      yield emptyFrame(tick);
      tick += 1n;
    }
    if (frame.tick >= end) return;
    yield frame;
    tick = frame.tick + 1n;
  }
  while (tick < end) {
    yield emptyFrame(tick);
    tick += 1n;
  }
}
