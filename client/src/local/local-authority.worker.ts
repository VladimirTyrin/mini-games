/// <reference lib="webworker" />
// Runs the match authority (rules, bots, randomness) off the main thread so a minimax search
// never janks rendering.

import init, { LocalHost } from "@/wasm/pkg/mg_wasm";
import wasmUrl from "@/wasm/pkg/mg_wasm_bg.wasm?url";

import { toBinary } from "@bufbuild/protobuf";

import { commonRefusal } from "@/match/common-refusal";
import { CommonRefusal, RefusalSchema } from "@/proto/minigames/v2/refusal_pb";

import type { FromWorker, ToWorker } from "./worker-protocol";

declare const self: DedicatedWorkerGlobalScope;

let host: LocalHost | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;
let paused = false;
/**
 * A turn-based bot moves no sooner than this after its turn began, however fast it decided: a
 * bot answering at once, or bots playing each other, would be too fast to follow. The server
 * waits as long (`Timing::bot_move_min`).
 */
const BOT_MOVE_MIN_MS = 700;
let botMove: ReturnType<typeof setTimeout> | null = null;
/** Watching: the seats' secrets as last sent, so only changes go out. */
let watchedSeats = 0;
let sentSpectatorSecrets: string[] = [];

/** A refusal of the match itself, not of the rules (not running, paused), encoded. */
function encodedCommonRefusal(reason: CommonRefusal): Uint8Array {
  return toBinary(RefusalSchema, commonRefusal(reason));
}

/** For the console only: the worker has no dictionaries (nothing here reaches the screen). */
function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function post(message: FromWorker, transfer: Transferable[] = []): void {
  self.postMessage(message, transfer);
}

function sealAndPost(): void {
  if (!host) return;
  const frame = host.sealFrame();
  post({ type: "frame", frame }, [frame.buffer]);
  postSpectatorSecrets();
  if (host.isFinished()) {
    stopTicker();
    post({ type: "ended" });
  }
}

/** A spectator sees the seats' secrets the game shows (both fleets), sent as they change. */
function postSpectatorSecrets(): void {
  for (let slot = 0; slot < watchedSeats && host; slot++) {
    const secret = host.spectatorSeatSecret(slot);
    const key = secret.join(",");
    if (secret.length === 0 || sentSpectatorSecrets[slot] === key) continue;
    sentSpectatorSecrets[slot] = key;
    post({ type: "spectatorSecret", slot, secret }, [secret.buffer]);
  }
}

function botIsOnTurn(): boolean {
  return host !== null && !paused && !host.isFinished() && host.isWaitingForBot();
}

function driveBots(): void {
  if (botMove !== null || !botIsOnTurn()) return;
  botMove = setTimeout(() => {
    botMove = null;
    // Paused or stopped meanwhile: a resume drives the bots again.
    if (!botIsOnTurn()) return;
    host?.runBots();
    sealAndPost();
    driveBots();
  }, BOT_MOVE_MIN_MS);
}

function startTicker(tickMs: number): void {
  stopTicker();
  ticker = setInterval(() => {
    if (!host || paused) return;
    host.runBots();
    sealAndPost();
  }, tickMs);
}

function stopTicker(): void {
  if (ticker) clearInterval(ticker);
  ticker = null;
}

async function handle(message: ToWorker): Promise<void> {
  switch (message.type) {
    case "start": {
      await init({ module_or_path: wasmUrl });
      host = new LocalHost(message.header, message.seedHigh, message.seedLow);
      watchedSeats = message.watchedSeats;
      sentSpectatorSecrets = [];
      sealAndPost();
      const tickMs = host.tickMs();
      if (tickMs > 0) startTicker(tickMs);
      else driveBots();
      break;
    }
    case "submit": {
      if (!host) {
        post({
          type: "rejected",
          requestId: message.requestId,
          refusal: encodedCommonRefusal(CommonRefusal.GAME_OVER),
        });
        break;
      }
      if (paused) {
        post({
          type: "rejected",
          requestId: message.requestId,
          refusal: encodedCommonRefusal(CommonRefusal.PAUSED),
        });
        break;
      }
      try {
        const refusal = host.submit(message.slot, message.command);
        if (refusal) {
          post({ type: "rejected", requestId: message.requestId, refusal });
          break;
        }
        post({ type: "accepted", requestId: message.requestId });
        if (host.tickMs() === 0) {
          sealAndPost();
          driveBots();
        }
      } catch (error) {
        console.error("local authority refused a malformed command:", describe(error));
        post({
          type: "rejected",
          requestId: message.requestId,
          refusal: encodedCommonRefusal(CommonRefusal.MALFORMED),
        });
      }
      break;
    }
    case "submitSecret": {
      if (!host) {
        post({
          type: "rejected",
          requestId: message.requestId,
          refusal: encodedCommonRefusal(CommonRefusal.GAME_OVER),
        });
        break;
      }
      try {
        const refusal = host.submitSecret(message.slot, message.command);
        if (refusal) {
          post({ type: "rejected", requestId: message.requestId, refusal });
          break;
        }
        post({ type: "accepted", requestId: message.requestId });
        const secret = host.seatSecret(message.slot);
        if (secret.length > 0) post({ type: "seatSecret", secret }, [secret.buffer]);
        // What the others may learn of it (the fleet is placed) goes out at once in a turn-based
        // game; a real-time one sends it with the next tick (sealing now would add a tick).
        if (host.tickMs() === 0) {
          sealAndPost();
          driveBots();
        }
      } catch (error) {
        console.error("local authority refused a malformed command:", describe(error));
        post({
          type: "rejected",
          requestId: message.requestId,
          refusal: encodedCommonRefusal(CommonRefusal.MALFORMED),
        });
      }
      break;
    }
    case "pause":
      paused = true;
      break;
    case "resume":
      paused = false;
      if (host && host.tickMs() === 0) driveBots();
      break;
    case "replay":
      // Answered either way: the page waits for it.
      post({ type: "replay", requestId: message.requestId, replay: host ? host.replayFile() : null });
      break;
    case "stop":
      stopTicker();
      if (botMove !== null) clearTimeout(botMove);
      host?.free();
      host = null;
      self.close();
      break;
  }
}

// One message at a time, in order: a command sent while "start" still loads the WASM waits for
// it instead of finding no match and being dropped.
let handled: Promise<void> = Promise.resolve();
self.onmessage = (event: MessageEvent<ToWorker>) => {
  handled = handled
    .then(() => handle(event.data))
    .catch((error: unknown) => post({ type: "failed", message: describe(error) }));
};
