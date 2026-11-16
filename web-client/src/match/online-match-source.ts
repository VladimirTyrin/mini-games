import { create } from "@bufbuild/protobuf";

import { GameCommandSchema } from "@/proto/minigames/v2/game_pb";
import type { MatchHeader } from "@/proto/minigames/v2/match_pb";
import { Emitter } from "@/net/emitter";
import type { Session } from "@/net/session";

import type { GameCommandInit, MatchSource, MatchSourceEvents } from "./match-source";

/** Server-authoritative match over the signed-in session. */
export class OnlineMatchSource implements MatchSource {
  readonly kind = "online";
  private readonly events = new Emitter<MatchSourceEvents>();
  private readonly unsubscribe: (() => void)[] = [];

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
        if (paused.matchId === this.matchId && paused.pause) this.events.emit("paused", paused.pause);
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
    await this.session.request({
      case: "submitCommand",
      value: { matchId: this.matchId, command: create(GameCommandSchema, command) },
    });
  }

  requestCatchUp(): void {
    this.session.request({ case: "requestCatchUp", value: { matchId: this.matchId } }).catch(() => undefined);
  }

  async voteDrop(slot: number): Promise<void> {
    await this.session.request({ case: "voteDropPlayer", value: { matchId: this.matchId, slot } });
  }

  leave(): void {
    for (const unsubscribe of this.unsubscribe) unsubscribe();
    this.unsubscribe.length = 0;
  }

  async replayFile(): Promise<Uint8Array | null> {
    const reply = await this.session.request({ case: "requestReplay", value: { matchId: this.matchId } });
    return reply.case === "replayData" ? reply.value.replayFile : null;
  }
}
