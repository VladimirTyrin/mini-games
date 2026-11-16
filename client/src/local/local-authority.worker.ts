/// <reference lib="webworker" />
// Runs the match authority (rules, bots, randomness) off the main thread so a minimax search
// never janks rendering.

import init, { LocalHost } from "@/wasm/pkg/mg_wasm";
import wasmUrl from "@/wasm/pkg/mg_wasm_bg.wasm?url";

import type { FromWorker, ToWorker } from "./worker-protocol";
import { errorMessage } from "@/util/error-message";

declare const self: DedicatedWorkerGlobalScope;

let host: LocalHost | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;
let paused = false;

function post(message: FromWorker, transfer: Transferable[] = []): void {
  self.postMessage(message, transfer);
}

function sealAndPost(): void {
  if (!host) return;
  const frame = host.sealFrame();
  post({ type: "frame", frame }, [frame.buffer]);
  if (host.isFinished()) {
    stopTicker();
    post({ type: "ended" });
  }
}

function driveBots(): void {
  while (host && !paused && !host.isFinished() && host.isWaitingForBot()) {
    host.runBots();
    sealAndPost();
  }
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
      sealAndPost();
      const tickMs = host.tickMs();
      if (tickMs > 0) startTicker(tickMs);
      else driveBots();
      break;
    }
    case "submit": {
      if (!host) return;
      try {
        host.submit(message.slot, message.command);
        post({ type: "accepted", requestId: message.requestId });
        if (host.tickMs() === 0) {
          sealAndPost();
          driveBots();
        }
      } catch (error) {
        post({
          type: "rejected",
          requestId: message.requestId,
          message: errorMessage(error),
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
      if (host) post({ type: "replay", requestId: message.requestId, replay: host.replayFile() });
      break;
    case "stop":
      stopTicker();
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
    .catch((error: unknown) => post({ type: "failed", message: errorMessage(error) }));
};
