import { type StackAttackWorker, StackAttackWorkerStatus } from "@/proto/minigames/v2/stack_attack_pb";
import { pulse, segment } from "@/render/easing";
import { ParticleField } from "@/render/particles";
import { prefersReducedMotion } from "@/render/reduced-motion";

import { BOX_FRAGMENT_COLORS } from "./box-sprite";
import { CRUSH_SETTLE_MS, crushedScaleY, settledCrushBoxY, squashedScaleX } from "./crush";
import { type Facing, FacingTracker } from "./facing";
import { headHitProgress } from "./head-hit";
import type { StackAttackTransition } from "./stack-attack-transition";
import { WalkTween } from "./walk-tween";
import { type GridPoint, stepWalkProgress, stepX, stepY, type WorkerStep } from "./worker-motion";
import { workerColor, workerShade } from "./worker-palette";

const LANDING_SQUASH_MS = 110;
const LANDING_SQUASH = 0.15;
/** A push pose outlasts its tick a little so pushing a row of crates reads as one long shove. */
const PUSH_POSE_TICKS = 1.25;
/** A crushed worker stays visible this long while the match goes on, fading out at the end. */
const CRUSHED_VISIBLE_MS = 1600;
const CRUSHED_FADE_MS = 400;
const SHAKE_MS = 220;
const SHAKE_CELLS = 0.08;
const BREATH_PERIOD_MS = 1500;
const BLINK_PERIOD_MS = 3700;
const BLINK_MS = 120;
const DUST = "#cbd5e1";
/** The hard hat hitting a crate: a quick squash and dip in the jump pose. */
const HEAD_BUMP_MS = 140;
const HEAD_BUMP_SQUASH = 0.14;
const HEAD_BUMP_DIP_CELLS = 0.06;
/** Where a broken crate's quarters start (from its centre, in cells) and which way they fly. */
const SHATTER_QUARTERS: readonly { dx: number; dy: number; angle: number }[] = [
  { dx: -0.22, dy: 0.22, angle: Math.PI / 2 + 0.6 },
  { dx: 0.22, dy: 0.22, angle: Math.PI / 2 - 0.6 },
  { dx: -0.22, dy: -0.22, angle: Math.PI - 0.45 },
  { dx: 0.22, dy: -0.22, angle: 0.45 },
];

export type WorkerEyes = "open" | "closed" | "dazed";

/** Everything the sprite needs to draw one worker; one reused object per seat. */
export interface WorkerPose {
  /** World column of the feet's centre (cell x + 0.5) and world height of the soles. */
  x: number;
  y: number;
  facing: Facing;
  /** Walk cycle frame 0..3, or -1 when standing. */
  walkFrame: number;
  pushing: boolean;
  airborne: boolean;
  scaleX: number;
  scaleY: number;
  /** -1..1, idle breathing. */
  breath: number;
  eyes: WorkerEyes;
  alpha: number;
  color: string;
  shade: string;
}

interface WorkerTrack {
  step: WorkerStep;
  /** Vertical move of the previous step, to tell a fall that continues from one that starts. */
  lastStepDy: number;
  landedAt: number;
  bumpedAt: number;
  pushFrom: number;
  pushUntil: number;
  crushedAt: number;
  crushBoxId: number;
  crushFeet: GridPoint;
  readonly pose: WorkerPose;
}

const NEVER = Number.NEGATIVE_INFINITY;
const ORIGIN: GridPoint = { x: 0, y: 0 };

function newTrack(slot: number): WorkerTrack {
  return {
    step: { from: ORIGIN, to: ORIGIN, continuesFall: false },
    lastStepDy: 0,
    landedAt: NEVER,
    bumpedAt: NEVER,
    pushFrom: NEVER,
    pushUntil: NEVER,
    crushedAt: NEVER,
    crushBoxId: -1,
    crushFeet: ORIGIN,
    pose: {
      x: 0,
      y: 0,
      facing: 1,
      walkFrame: -1,
      pushing: false,
      airborne: false,
      scaleX: 1,
      scaleY: 1,
      breath: 0,
      eyes: "open",
      alpha: 1,
      color: workerColor(slot),
      shade: workerShade(slot),
    },
  };
}

/** A box broken by a head hit, drawn falling until the hard hat reaches it. */
interface BreakingBox {
  readonly startRow: number;
  readonly from: number;
  readonly hitAt: number;
}

const isAlive = (worker: StackAttackWorker | undefined) => worker?.status === StackAttackWorkerStatus.ALIVE;

/**
 * Client-side animation state of the workers across frames: facing, falls that continue, landing
 * squash, push pose, crush, particles — and the own worker's predicted walk.
 *
 * The board draws one tick behind the authority, so a press would show only after up to two
 * ticks. The own worker's horizontal position therefore comes from a `WalkTween` started the
 * moment a move is sent (when the target cell is free); frames that confirm it leave the tween
 * alone, and once no command of ours is in flight any disagreement is walked off smoothly.
 */
export class WorkerAnimator {
  readonly particles = new ParticleField(96, -18);
  private readonly tracks: WorkerTrack[] = [];
  private readonly facings = new FacingTracker();
  private readonly breakingBoxes = new Map<number, BreakingBox>();
  private readonly ownWalk: WalkTween;
  private shakeAt = NEVER;

  constructor(
    private readonly tickMs: number,
    private readonly ownSlot: number | null,
  ) {
    this.ownWalk = new WalkTween(tickMs);
  }

  /** A new frame is shown from `transition.at`; `ownCommandsInFlight` from the input pacer. */
  observe(transition: StackAttackTransition, now: number, ownCommandsInFlight: boolean): void {
    const { view, previous, at } = transition;
    if (!previous) this.reset();
    view.workers.forEach((after, seat) => {
      const track = (this.tracks[seat] ??= newTrack(seat));
      const before = previous?.workers[seat];
      const to = after.feet ?? track.step.to;
      const from = isAlive(before) && isAlive(after) && before?.feet ? before.feet : to;
      track.step = { from, to, continuesFall: track.lastStepDy < 0 };
      track.lastStepDy = to.y - from.y;
      this.facings.moved(seat, to.x - from.x);
    });
    for (const [boxId, breaking] of this.breakingBoxes)
      if (breaking.hitAt < at) this.breakingBoxes.delete(boxId);
    for (const { event } of view.events) {
      switch (event.case) {
        case "workerLanded": {
          const track = this.tracks[event.value.slot];
          if (!track) break;
          // Landing after a sideways move (onto a crate) touches down when the move ends.
          const movedThisTick = track.step.from.x !== track.step.to.x;
          track.landedAt = at + (movedThisTick ? this.tickMs : 0);
          this.dust(track.step.to, track.landedAt);
          break;
        }
        case "boxPushed": {
          const track = this.tracks[event.value.bySlot];
          if (!track) break;
          track.pushFrom = at;
          track.pushUntil = at + this.tickMs * PUSH_POSE_TICKS;
          this.facings.moved(event.value.bySlot, event.value.toX - event.value.fromX);
          break;
        }
        case "boxBroken": {
          const track = this.tracks[event.value.bySlot];
          const cell = event.value.cell;
          if (!track || !cell) break;
          const hitProgress = headHitProgress(cell.y, track.step);
          const hitAt = at + hitProgress * this.tickMs;
          this.breakingBoxes.set(event.value.boxId, { startRow: cell.y, from: at, hitAt });
          track.bumpedAt = hitAt;
          this.shatter(cell.x, cell.y - hitProgress, hitAt);
          break;
        }
        case "workerCrushed": {
          const track = this.tracks[event.value.slot];
          if (!track) break;
          track.crushedAt = at;
          track.crushBoxId = event.value.boxId;
          track.crushFeet = track.step.to;
          this.crushBurst(track, at + this.tickMs);
          break;
        }
        default:
          break;
      }
    }
    this.reconcileOwnWalk(transition, now, ownCommandsInFlight);
  }

  /** The player pressed left or right: the own worker looks that way even if it cannot step. */
  faceOwn(facing: Facing): void {
    if (this.ownSlot !== null) this.facings.turned(this.ownSlot, facing);
  }

  /** The own worker was just sent towards column `targetX`, which is free in the latest view. */
  predictWalk(targetX: number, now: number): void {
    if (this.ownSlot === null || this.ownWalk.target === null) return;
    this.facings.moved(this.ownSlot, targetX - this.ownWalk.target);
    this.ownWalk.walkTo(targetX, now);
  }

  /** The pose of `seat` at `now`, or `null` when nothing should be drawn for it. */
  poseAt(seat: number, transition: StackAttackTransition, progress: number, now: number): WorkerPose | null {
    const track = this.tracks[seat];
    const worker = transition.view.workers[seat];
    if (!track || !worker) return null;
    const pose = track.pose;
    const reducedMotion = prefersReducedMotion();
    pose.facing = this.facings.facing(seat);
    pose.breath = reducedMotion ? 0 : Math.sin(((now / BREATH_PERIOD_MS + seat * 0.37) % 1) * Math.PI * 2);
    pose.eyes = (now + seat * 977) % BLINK_PERIOD_MS < BLINK_MS ? "closed" : "open";
    pose.alpha = 1;
    pose.scaleX = 1;
    pose.scaleY = 1;
    if (worker.status === StackAttackWorkerStatus.CRUSHED)
      return this.crushedPose(track, transition, progress, now);
    if (worker.status !== StackAttackWorkerStatus.ALIVE) return null;

    const { step } = track;
    const own = seat === this.ownSlot && this.ownWalk.target !== null;
    pose.x = (own ? this.ownWalk.valueAt(now) : stepX(step, progress)) + 0.5;
    pose.y = stepY(step, progress);
    const walking = own ? this.ownWalk.isMovingAt(now) : step.from.x !== step.to.x && progress < 1;
    const walkProgress = own ? this.ownWalk.progressAt(now) : stepWalkProgress(step, progress);
    const column = own ? (this.ownWalk.target ?? 0) : step.to.x;
    pose.walkFrame = walking ? (column & 1) * 2 + (walkProgress < 0.5 ? 0 : 1) : -1;
    pose.airborne = step.from.y !== step.to.y && progress < 1;
    pose.pushing = now >= track.pushFrom && now < track.pushUntil;
    if (!reducedMotion && now >= track.landedAt && now < track.landedAt + LANDING_SQUASH_MS) {
      const squash = LANDING_SQUASH * pulse((now - track.landedAt) / LANDING_SQUASH_MS);
      pose.scaleY = 1 - squash;
      pose.scaleX = 1 + squash * 0.8;
    }
    if (!reducedMotion && now >= track.bumpedAt && now < track.bumpedAt + HEAD_BUMP_MS) {
      const bump = pulse((now - track.bumpedAt) / HEAD_BUMP_MS);
      pose.scaleY = 1 - HEAD_BUMP_SQUASH * bump;
      pose.scaleX = 1 + HEAD_BUMP_SQUASH * 0.6 * bump;
      pose.y -= HEAD_BUMP_DIP_CELLS * bump;
    }
    return pose;
  }

  /**
   * Row of the bottom edge of `boxId` at `now`: its tick motion plus, for the box that ended the
   * match by crushing a worker, the settle onto the flattened worker after the last tick.
   */
  boxRowAt(transition: StackAttackTransition, boxId: number, progress: number, now: number): number | null {
    const row = transition.boxRowAt(boxId, progress);
    const crush = transition.finalCrush;
    if (row === null || !crush || crush.boxId !== boxId) return row;
    const settleFrom = transition.at + this.tickMs;
    return settledCrushBoxY(row, crush.feet.y, segment(now, settleFrom, settleFrom + CRUSH_SETTLE_MS));
  }

  /** Row of the bottom edge of a box broken this tick, or `null` once the hard hat has hit it. */
  breakingBoxRowAt(boxId: number, now: number): number | null {
    const breaking = this.breakingBoxes.get(boxId);
    if (!breaking || now >= breaking.hitAt) return null;
    return breaking.startRow - segment(now, breaking.from, breaking.from + this.tickMs);
  }

  /** Vertical screen shake in cells (impact of a crush). */
  shakeOffset(now: number): number {
    if (prefersReducedMotion() || now < this.shakeAt || now >= this.shakeAt + SHAKE_MS) return 0;
    const progress = (now - this.shakeAt) / SHAKE_MS;
    return SHAKE_CELLS * (1 - progress) * Math.sin(progress * Math.PI * 6);
  }

  /** Something keeps moving after the last tick's motion ended. */
  hasEffectsAt(now: number): boolean {
    if (this.ownWalk.isMovingAt(now) || this.particles.isActive(now) || now < this.shakeAt + SHAKE_MS)
      return true;
    return this.tracks.some(
      (track) =>
        now < track.landedAt + LANDING_SQUASH_MS ||
        now < track.bumpedAt + HEAD_BUMP_MS ||
        now < track.pushUntil ||
        now < track.crushedAt + CRUSHED_VISIBLE_MS + this.tickMs,
    );
  }

  private crushedPose(
    track: WorkerTrack,
    transition: StackAttackTransition,
    progress: number,
    now: number,
  ): WorkerPose | null {
    // The crush that ended the match stays; earlier ones fade while the others play on.
    const endedMatch = transition.view.endReason !== undefined && track.crushedAt === transition.at;
    const shownFor = now - track.crushedAt;
    if (!endedMatch && !(shownFor < CRUSHED_VISIBLE_MS)) return null;
    const pose = track.pose;
    pose.x = track.crushFeet.x + 0.5;
    pose.y = track.crushFeet.y;
    pose.walkFrame = -1;
    pose.pushing = false;
    pose.airborne = false;
    const boxRow = this.boxRowAt(transition, track.crushBoxId, progress, now);
    pose.scaleY = crushedScaleY(boxRow ?? pose.y, pose.y);
    pose.scaleX = squashedScaleX(pose.scaleY);
    // Dazed only once the crate hits: until then the worker still looks up, unaware.
    if (pose.scaleY < 0.95) pose.eyes = "dazed";
    pose.alpha = endedMatch
      ? 1
      : 1 - segment(shownFor, CRUSHED_VISIBLE_MS - CRUSHED_FADE_MS, CRUSHED_VISIBLE_MS);
    return pose;
  }

  private reconcileOwnWalk(
    transition: StackAttackTransition,
    now: number,
    ownCommandsInFlight: boolean,
  ): void {
    if (this.ownSlot === null) return;
    const own = transition.view.workers[this.ownSlot];
    if (!own?.feet || !isAlive(own)) return;
    if (this.ownWalk.target === null) this.ownWalk.place(own.feet.x);
    // While commands are in flight the view lags behind what was predicted; judge it afterwards.
    else if (!ownCommandsInFlight && this.ownWalk.target !== own.feet.x)
      this.ownWalk.walkTo(own.feet.x, Math.max(now, transition.at));
  }

  private reset(): void {
    this.tracks.length = 0;
    this.facings.reset();
    this.ownWalk.reset();
    this.particles.clear();
    this.breakingBoxes.clear();
    this.shakeAt = NEVER;
  }

  private dust(feet: GridPoint, at: number): void {
    if (prefersReducedMotion()) return;
    for (const direction of [-1, 1]) {
      this.particles.emit({
        x: feet.x + 0.5 + direction * 0.2,
        y: 0.05 + feet.y,
        count: 2,
        speed: 1.6,
        angle: direction > 0 ? 0.35 : Math.PI - 0.35,
        spread: 0.5,
        size: 0.09,
        lifeMs: 260,
        color: DUST,
        at,
      });
    }
  }

  /** A crate in column `x`, its bottom edge at `row`, bursts: a chunk and splinters per quarter. */
  private shatter(x: number, row: number, at: number): void {
    if (prefersReducedMotion()) return;
    SHATTER_QUARTERS.forEach(({ dx, dy, angle }, quarter) => {
      const piece = { x: x + 0.5 + dx, y: row + 0.5 + dy, angle, spread: 0.7, lifeMs: 620, at };
      const chunkColor = BOX_FRAGMENT_COLORS[quarter % 2] ?? DUST;
      this.particles.emit({ ...piece, count: 1, speed: 3, size: 0.34, color: chunkColor });
      this.particles.emit({
        ...piece,
        count: 2,
        speed: 4.5,
        size: 0.12,
        color: BOX_FRAGMENT_COLORS[2] ?? DUST,
      });
    });
  }

  private crushBurst(track: WorkerTrack, at: number): void {
    this.shakeAt = at;
    if (prefersReducedMotion()) return;
    const feet = track.crushFeet;
    for (const direction of [-1, 1]) {
      const burst = {
        x: feet.x + 0.5 + direction * 0.35,
        y: feet.y + 0.2,
        speed: 4.5,
        angle: direction > 0 ? 0.6 : Math.PI - 0.6,
        spread: 0.9,
        lifeMs: 650,
        at,
      };
      this.particles.emit({ ...burst, count: 5, size: 0.13, color: track.pose.color });
      this.particles.emit({ ...burst, count: 4, size: 0.1, color: DUST });
    }
  }
}
