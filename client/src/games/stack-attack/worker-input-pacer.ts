export type WalkDirection = "left" | "right";
export type WorkerAction = WalkDirection | "jump";

/** Presses that arrive faster than one per tick wait here; older ones beyond this are dropped. */
export const MAX_BUFFERED_PRESSES = 2;

/**
 * Turns presses and holds into commands at the pace the game consumes them: one per tick.
 *
 * The authority executes one queued command per worker per tick, so sending more than that only
 * builds a backlog the player feels as lag (and the 3-slot queue silently drops the excess). The
 * pacer keeps at most one command in the pipeline — sent but not yet acknowledged, acknowledged
 * but not yet seen in a frame, or still queued in the latest view — and sends the next one when
 * the pipeline empties: a buffered press first, otherwise the held direction (hold to walk).
 *
 * Relies on the authority acknowledging a command before it seals the frame that contains it,
 * over one ordered channel (true for the server and the local worker): the first frame after an
 * acknowledgement has applied the command.
 */
export class WorkerInputPacer {
  /** Held directions, latest press last: it wins until released. */
  private readonly held: WalkDirection[] = [];
  private readonly buffered: WorkerAction[] = [];
  private unacknowledged = 0;
  private acknowledgedAwaitingFrame = 0;
  private queuedAtAuthority = 0;

  /** `send` returns false when the command could not be sent (e.g. the worker is dead). */
  constructor(private readonly send: (action: WorkerAction) => boolean) {}

  get hasCommandsInFlight(): boolean {
    return this.unacknowledged + this.acknowledgedAwaitingFrame + this.queuedAtAuthority > 0;
  }

  get heldDirection(): WalkDirection | null {
    return this.held.at(-1) ?? null;
  }

  press(action: WorkerAction): void {
    if (action !== "jump") {
      this.release(action);
      this.held.push(action);
    }
    if (!this.hasCommandsInFlight && this.buffered.length === 0) {
      this.dispatch(action);
      return;
    }
    this.buffered.push(action);
    if (this.buffered.length > MAX_BUFFERED_PRESSES) this.buffered.shift();
  }

  release(direction: WalkDirection): void {
    const index = this.held.indexOf(direction);
    if (index >= 0) this.held.splice(index, 1);
  }

  /** Focus lost, worker dead, match over: forget everything that was pressed. */
  reset(): void {
    this.held.length = 0;
    this.buffered.length = 0;
  }

  acknowledged(): void {
    this.unacknowledged = Math.max(0, this.unacknowledged - 1);
    this.acknowledgedAwaitingFrame++;
  }

  /** Refused commands never reach a frame; the next frame resumes sending. */
  rejected(): void {
    this.unacknowledged = Math.max(0, this.unacknowledged - 1);
  }

  /** A new frame was applied; `queuedCommands` is this worker's queue in its view. */
  frameArrived(queuedCommands: number): void {
    this.acknowledgedAwaitingFrame = 0;
    this.queuedAtAuthority = queuedCommands;
  }

  /** Sends the next command if the pipeline is empty; call after `frameArrived`. */
  pump(): void {
    if (this.hasCommandsInFlight) return;
    const action = this.buffered.shift() ?? this.heldDirection;
    if (action) this.dispatch(action);
  }

  private dispatch(action: WorkerAction): void {
    if (this.send(action)) this.unacknowledged++;
  }
}
