import {
  type StackAttackBox,
  StackAttackCraneState,
  StackAttackWorkerStatus,
} from "@/proto/minigames/v2/stack_attack_pb";
import { lerp } from "@/render/tick-progress";
import type { CanvasSize } from "@/render/use-canvas";

import { drawBox } from "./box-sprite";
import { StackAttackLayout } from "./stack-attack-layout";
import type { StackAttackTransition } from "./stack-attack-transition";
import type { WorkerAnimator } from "./worker-animator";
import { WORKER_DRAWN_HEIGHT_CELLS } from "./worker-dimensions";
import { drawWorkerSprite, drawYouMarker } from "./worker-sprite";

export interface StackAttackFrame {
  readonly transition: StackAttackTransition;
  readonly animator: WorkerAnimator;
  /** 0..1 through the tick from `transition.previous` to `transition.view`. */
  readonly progress: number;
  readonly now: number;
}

const BACKGROUND = "#2d3748";
const BACK_WALL_LINE = "rgb(255 255 255 / 0.035)";
const RAIL = "#1a202c";
const TROLLEY = "#94a3b8";
const TROLLEY_DARK = "#475569";
const CABLE_CORE = "#e2e8f0";
const CABLE_EDGE = "#0f172a";
const CEILING = "rgb(248 113 113 / 0.55)";
const FLOOR = "#4a5568";
const FLOOR_EDGE = "#718096";
const CLEARED_ROW_FLASH = "rgb(255 255 255 / 0.7)";
/** Wheel positions across a trolley, in cells. */
const TROLLEY_WHEELS: readonly number[] = [0.28, 0.72];

interface Painter {
  readonly context: CanvasRenderingContext2D;
  readonly layout: StackAttackLayout;
  readonly frame: StackAttackFrame;
}

export function drawStackAttackScene(
  context: CanvasRenderingContext2D,
  canvas: CanvasSize,
  frame: StackAttackFrame,
): void {
  context.clearRect(0, 0, canvas.width, canvas.height);
  const { view } = frame.transition;
  if (view.fieldWidth <= 0 || view.fieldHeight <= 0) return;
  const layout = StackAttackLayout.fit(canvas, view.fieldWidth, view.fieldHeight);
  const painter: Painter = { context, layout, frame };

  context.save();
  // Cranes enter and leave through off-field columns; the clip hides them there.
  context.beginPath();
  context.rect(layout.originX, layout.originY, layout.widthPx, layout.bottomY - layout.originY);
  context.clip();
  drawScenery(painter);
  context.translate(0, Math.round(frame.animator.shakeOffset(frame.now) * layout.cellSize));
  drawWorkers(painter, "crushed");
  drawBoxes(painter);
  drawClearedRows(painter);
  drawCranes(painter);
  drawWorkers(painter, "alive");
  frame.animator.particles.draw(context, layout, frame.now);
  context.restore();
}

function drawScenery({ context, layout }: Painter): void {
  const cell = layout.cellSize;
  context.fillStyle = BACKGROUND;
  context.fillRect(layout.originX, layout.originY, layout.widthPx, layout.floorY - layout.originY);
  // Faint column lines on the back wall help judge where a crate will land.
  context.fillStyle = BACK_WALL_LINE;
  for (let column = 1; column < layout.fieldWidth; column++)
    context.fillRect(layout.cellLeft(column), layout.ceilingY, 1, layout.floorY - layout.ceilingY);
  context.fillStyle = RAIL;
  context.fillRect(layout.originX, layout.originY, layout.widthPx, cell * 0.15);
  context.fillStyle = FLOOR;
  context.fillRect(layout.originX, layout.floorY, layout.widthPx, layout.bottomY - layout.floorY);
  context.fillStyle = FLOOR_EDGE;
  context.fillRect(layout.originX, layout.floorY, layout.widthPx, Math.max(1, cell * 0.06));
  context.strokeStyle = CEILING;
  context.lineWidth = 2;
  context.setLineDash([cell * 0.25, cell * 0.2]);
  context.beginPath();
  context.moveTo(layout.originX, layout.ceilingY);
  context.lineTo(layout.originX + layout.widthPx, layout.ceilingY);
  context.stroke();
  context.setLineDash([]);
}

function drawMovingBox({ context, layout, frame }: Painter, box: StackAttackBox): void {
  const { transition, animator, progress, now } = frame;
  const start = transition.boxStartOf(box);
  const row = animator.boxRowAt(transition, box.id, progress, now);
  if (!start || !box.cell || row === null) return;
  const column = lerp(start.x, box.cell.x, progress);
  drawBox(context, layout.cellLeft(column), layout.cellTop(row), layout.cellSize, box.patternId);
}

function drawBoxes(painter: Painter): void {
  const { context, layout, frame } = painter;
  const { transition, progress, now } = frame;
  for (const { after } of transition.boxes.kept) drawMovingBox(painter, after);
  for (const box of transition.boxes.appeared) drawMovingBox(painter, box);
  for (const box of transition.boxes.vanished) {
    if (!transition.brokenBoxIds.has(box.id) || !box.cell) continue;
    // A broken box keeps falling until the rising worker's head reaches it.
    const row = frame.animator.breakingBoxRowAt(box.id, now);
    if (row !== null)
      drawBox(context, layout.cellLeft(box.cell.x), layout.cellTop(row), layout.cellSize, box.patternId);
  }
  // Boxes of cleared rows fade out under the row flash.
  context.save();
  context.globalAlpha = 1 - progress;
  for (const box of transition.boxes.vanished) {
    if (box.cell && !transition.brokenBoxIds.has(box.id))
      drawBox(
        context,
        layout.cellLeft(box.cell.x),
        layout.cellTop(box.cell.y),
        layout.cellSize,
        box.patternId,
      );
  }
  context.restore();
}

function drawClearedRows({ context, layout, frame }: Painter): void {
  if (frame.transition.clearedRows.length === 0) return;
  context.save();
  context.globalAlpha = 1 - frame.progress;
  context.fillStyle = CLEARED_ROW_FLASH;
  for (const row of frame.transition.clearedRows)
    context.fillRect(layout.originX, layout.cellTop(row), layout.widthPx, layout.cellSize);
  context.restore();
}

function drawCranes(painter: Painter): void {
  const { transition, progress } = painter.frame;
  for (const { before, after } of transition.cranes.kept)
    drawCrane(painter, lerp(before.x, after.x, progress), after.state, after.patternId);
  for (const crane of transition.cranes.appeared) drawCrane(painter, crane.x, crane.state, crane.patternId);
}

function drawCrane(
  { context, layout, frame }: Painter,
  x: number,
  state: StackAttackCraneState,
  pattern: number,
): void {
  const cell = layout.cellSize;
  const left = layout.cellLeft(x);
  const centre = Math.round(left + cell / 2);
  const carriedTop = layout.cellTop(frame.transition.view.fieldHeight);
  const trolleyTop = layout.originY + cell * 0.08;
  const trolleyHeight = cell * 0.36;

  // The cable: a light core with dark edges stays visible over the wall and over crates.
  const cableTop = trolleyTop + trolleyHeight;
  const cableBottom = carriedTop + (state === StackAttackCraneState.MOVING ? 0 : cell * 0.1);
  const cableWidth = Math.max(2, Math.round(cell * 0.08));
  context.fillStyle = CABLE_EDGE;
  context.fillRect(centre - cableWidth / 2 - 1, cableTop, cableWidth + 2, cableBottom - cableTop);
  context.fillStyle = CABLE_CORE;
  context.fillRect(centre - cableWidth / 2, cableTop, cableWidth, cableBottom - cableTop);

  context.fillStyle = TROLLEY_DARK;
  context.fillRect(left + cell * 0.1, trolleyTop, cell * 0.8, trolleyHeight);
  context.fillStyle = TROLLEY;
  context.fillRect(left + cell * 0.14, trolleyTop + cell * 0.04, cell * 0.72, trolleyHeight - cell * 0.12);
  context.fillStyle = CABLE_EDGE;
  for (const wheel of TROLLEY_WHEELS) {
    context.beginPath();
    context.arc(left + cell * wheel, trolleyTop, cell * 0.07, 0, Math.PI * 2);
    context.fill();
  }

  if (state === StackAttackCraneState.MOVING) {
    drawBox(context, left, carriedTop, cell, pattern);
    return;
  }
  // Empty hook after the drop.
  context.strokeStyle = CABLE_CORE;
  context.lineWidth = Math.max(2, cell * 0.07);
  context.beginPath();
  context.arc(centre, cableBottom + cell * 0.08, cell * 0.1, Math.PI * 1.5, Math.PI * 0.9);
  context.stroke();
}

function drawWorkers({ context, layout, frame }: Painter, which: "alive" | "crushed"): void {
  const { transition, animator, progress, now } = frame;
  const { view } = transition;
  const showYou = view.workers.length > 1;
  for (let seat = 0; seat < view.workers.length; seat++) {
    const isCrushed = view.workers[seat]?.status === StackAttackWorkerStatus.CRUSHED;
    if ((which === "crushed") !== isCrushed) continue;
    const pose = animator.poseAt(seat, transition, progress, now);
    if (!pose) continue;
    const footX = layout.pixelX(pose.x);
    const footY = layout.pixelY(pose.y);
    drawWorkerSprite(context, pose, footX, footY, layout.cellSize);
    if (showYou && !isCrushed && seat === view.you)
      drawYouMarker(
        context,
        footX,
        footY - WORKER_DRAWN_HEIGHT_CELLS * pose.scaleY * layout.cellSize,
        layout.cellSize,
      );
  }
}
