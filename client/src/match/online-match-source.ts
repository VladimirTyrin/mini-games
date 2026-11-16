import { create } from "@bufbuild/protobuf";

import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

import { GameCommandSchema } from "@/proto/minigames/v2/game_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";
import { Emitter } from "@/net/emitter";
import { RequestError } from "@/net/request-error";
import type { Session } from "@/net/session";

import { CommandRejectedError } from "./command-rejected-error";
import type { GameCommandInit, MatchSource, MatchSourceEvents } from "./match-source";

/** The server allows one catch-up per second per session. */
const CATCH_UP_RETRY_MS = 1_200;

/** Server-authoritative match over the signed-in session. */
export class OnlineMatchSource implements MatchSource {
  readonly kind = "online";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly unsubscribe: (() => void)[] = [];
  private isLeft = false;

  constructor(
    private readonly session: Session,
    readonly header: MatchHeader,
    readonly mySlot: number | null,
  ) {}

  private get matchId(): string {
    return this.header.matchId;
  }

  on<K extends keyof MatchSourceEvents>(
    event: K,
    handler: (payload: MatchSourceEvents[K]) => void,
  ): () => void {
    return this.events.on(event, handler);
  }

  start(): void {
    const session = this.session.events;
    this.unsubscribe.push(
      session.on("frame", (frame) => this.events.emit("frame", frame)),
      session.on("catchUp", (catchUp) => {
        if (catchUp.header?.matchId === this.matchId) this.events.emit("catchUp", catchUp);
      }),
      session.on("paused", (paused) => {
        if (paused.matchId !== this.matchId) return;
        // Servers before the full list only sent the seat that changed.
        const pauses = paused.pauses.length > 0 ? paused.pauses : paused.pause ? [paused.pause] : [];
        this.events.emit("paused", pauses);
      }),
      session.on("resumed", (resumed) => {
        if (resumed.matchId === this.matchId)
          this.events.emit("resumed", { countdownMs: resumed.countdownMs });
      }),
      session.on("ended", (ended) => {
        if (ended.matchId === this.matchId) this.events.emit("ended", ended.result ?? null);
      }),
      // After a reconnect the server resumes the seat; frames sent meanwhile are lost.
      session.on("signedIn", ({ resumed }) => {
        if (resumed) this.requestCatchUp();
      }),
    );
  }

  async submit(command: GameCommandInit): Promise<void> {
    try {
      await this.session.request({
        case: "submitCommand",
        value: { matchId: this.matchId, command: create(GameCommandSchema, command) },
      });
    } catch (error) {
      if (error instanceof RequestError && error.code === ErrorCode.COMMAND_REJECTED) {
        throw new CommandRejectedError(error.message, { cause: error });
      }
      throw error;
    }
  }

  requestCatchUp(): void {
    this.session.request({ case: "requestCatchUp", value: { matchId: this.matchId } }).catch((error) => {
      // The controller ignores frames until the catch-up arrives, so a refused request must be
      // retried; a lost connection triggers its own catch-up on resume.
      if (error instanceof RequestError && error.code === ErrorCode.RATE_LIMITED && !this.isLeft) {
        setTimeout(() => this.requestCatchUp(), CATCH_UP_RETRY_MS);
      }
    });
  }

  async voteDrop(slot: number): Promise<void> {
    await this.session.request({ case: "voteDropPlayer", value: { matchId: this.matchId, slot } });
  }

  leave(): void {
    this.isLeft = true;
    for (const unsubscribe of this.unsubscribe) unsubscribe();
    this.unsubscribe.length = 0;
  }

  async replayFile(): Promise<Uint8Array | null> {
    const reply = await this.session.request({ case: "requestReplay", value: { matchId: this.matchId } });
    return reply.case === "replayData" ? reply.value.replayFile : null;
  }
}
