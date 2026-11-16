import {
  type StackAttackBox,
  StackAttackCraneState,
  type StackAttackView,
  type StackAttackWorker,
  StackAttackWorkerStatus,
} from "@/proto/minigames/v2/stack_attack_pb";
import { lerp } from "@/render/tick-progress";
import type { CanvasSize } from "@/render/use-canvas";

import { drawBox } from "./box-sprite";
import { pairById } from "./pair-by-id";
import { StackAttackLayout } from "./stack-attack-layout";

export interface StackAttackScene {
  readonly view: StackAttackView;
  /** The view one tick earlier; rendering runs from it to `view` as `progress` goes 0..1. */
  readonly previous: StackAttackView | null;
  readonly progress: number;
}

const WORKER_COLORS = ["#48bb78", "#4299e1", "#ed64a6", "#ecc94b"] as const;

export function workerColor(slot: number): string {
  return WORKER_COLORS[slot % WORKER_COLORS.length] ?? WORKER_COLORS[0];
}

const BACKGROUND = "#2d3748";
const RAIL = "#1a202c";
const TROLLEY = "#718096";
const CABLE = "#a0aec0";
const CEILING = "rgb(248 113 113 / 0.55)";
const FLOOR = "#4a5568";
const CLEARED_ROW_FLASH = "rgb(255 255 255 / 0.7)";
const YOU_MARKER = "#ffffff";

interface CellPoint {
  readonly x: number;
  readonly y: number;
}

interface Frame {
  readonly context: CanvasRenderingContext2D;
  readonly layout: StackAttackLayout;
  readonly scene: StackAttackScene;
}

function between(from: CellPoint | undefined, to: CellPoint | undefined, progress: number): CellPoint | null {
  if (!to) return null;
  if (!from) return { x: to.x, y: to.y };
  return { x: lerp(from.x, to.x, progress), y: lerp(from.y, to.y, progress) };
}

export function drawStackAttackScene(
  context: CanvasRenderingContext2D,
  canvas: CanvasSize,
  scene: StackAttackScene,
): void {
  context.clearRect(0, 0, canvas.width, canvas.height);
  const { view } = scene;
  if (view.fieldWidth <= 0 || view.fieldHeight <= 0) return;
  const layout = StackAttackLayout.fit(canvas, view.fieldWidth, view.fieldHeight);
  const frame: Frame = { context, layout, scene };

  context.save();
  // Cranes enter and leave through off-field columns; the clip hides them there.
  context.beginPath();
  context.rect(layout.originX, layout.originY, layout.widthPx, layout.bottomY - layout.originY);
  context.clip();
  drawScenery(frame);
  drawBoxes(frame);
  drawClearedRows(frame);
  drawCranes(frame);
  drawWorkers(frame);
  context.restore();
}

function drawScenery({ context, layout }: Frame): void {
  context.fillStyle = BACKGROUND;
  context.fillRect(layout.originX, layout.originY, layout.widthPx, layout.floorY - layout.originY);
  context.fillStyle = RAIL;
  context.fillRect(layout.originX, layout.originY, layout.widthPx, layout.cellSize * 0.15);
  context.fillStyle = FLOOR;
  context.fillRect(layout.originX, layout.floorY, layout.widthPx, layout.bottomY - layout.floorY);
  context.strokeStyle = CEILING;
  context.lineWidth = 2;
  context.setLineDash([layout.cellSize * 0.25, layout.cellSize * 0.2]);
  context.beginPath();
  context.moveTo(layout.originX, layout.ceilingY);
  context.lineTo(layout.originX + layout.widthPx, layout.ceilingY);
  context.stroke();
  context.setLineDash([]);
}

function drawBoxes({ context, layout, scene }: Frame): void {
  const { view, previous, progress } = scene;
  const changes = pairById(previous?.boxes ?? [], view.boxes, (box) => box.id);
  const drawAt = (box: StackAttackBox, at: CellPoint | null) => {
    if (at) drawBox(context, layout.cellLeft(at.x), layout.cellTop(at.y), layout.cellSize, box.patternId);
  };
  for (const { before, after } of changes.kept) drawAt(after, between(before.cell, after.cell, progress));
  const droppedFrom = new Map<number, CellPoint>();
  for (const { event } of view.events) {
    if (event.case === "boxDropped")
      droppedFrom.set(event.value.boxId, { x: event.value.x, y: view.fieldHeight });
  }
  // A freshly released box falls out of its crane rather than popping up below it.
  for (const box of changes.appeared) drawAt(box, between(droppedFrom.get(box.id), box.cell, progress));
  // Boxes of cleared rows fade out under the row flash.
  context.save();
  context.globalAlpha = 1 - progress;
  for (const box of changes.vanished) drawAt(box, between(undefined, box.cell, progress));
  context.restore();
}

function drawClearedRows({ context, layout, scene }: Frame): void {
  context.save();
  context.globalAlpha = 1 - scene.progress;
  context.fillStyle = CLEARED_ROW_FLASH;
  for (const { event } of scene.view.events) {
    if (event.case !== "linesCleared") continue;
    for (const row of event.value.rows)
      context.fillRect(layout.originX, layout.cellTop(row), layout.widthPx, layout.cellSize);
  }
  context.restore();
}

function drawCranes({ context, layout, scene }: Frame): void {
  const { view, previous, progress } = scene;
  const cell = layout.cellSize;
  const changes = pairById(previous?.cranes ?? [], view.cranes, (crane) => crane.id);
  const cranes = [
    ...changes.kept.map(({ before, after }) => ({ crane: after, x: lerp(before.x, after.x, progress) })),
    ...changes.appeared.map((crane) => ({ crane, x: crane.x })),
  ];
  for (const { crane, x } of cranes) {
    const left = layout.cellLeft(x);
    const carriedTop = layout.cellTop(view.fieldHeight);
    context.fillStyle = TROLLEY;
    context.fillRect(left + cell * 0.15, layout.originY + cell * 0.1, cell * 0.7, cell * 0.4);
    context.fillStyle = CABLE;
    context.fillRect(
      left + cell * 0.45,
      layout.originY + cell * 0.5,
      cell * 0.1,
      carriedTop - layout.originY - cell * 0.4,
    );
    if (crane.state === StackAttackCraneState.MOVING)
      drawBox(context, left, carriedTop, cell, crane.patternId);
    else context.fillRect(left + cell * 0.3, carriedTop + cell * 0.05, cell * 0.4, cell * 0.12);
  }
}

function drawWorkers(frame: Frame): void {
  const { view, previous, progress } = frame.scene;
  view.workers.forEach((worker, seat) => {
    const before = previous?.workers[seat];
    const wasAlive = before?.status === StackAttackWorkerStatus.ALIVE;
    if (worker.status === StackAttackWorkerStatus.ALIVE) {
      const at = between(wasAlive ? before.feet : undefined, worker.feet, progress);
      if (at) drawWorker(frame, worker, at);
    } else if (worker.status === StackAttackWorkerStatus.CRUSHED && wasAlive) {
      // Fades during the tick it was crushed in; the crush that ended the game stays.
      const at = between(undefined, worker.feet ?? before.feet, progress);
      if (at) drawCrushedWorker(frame, worker, at, view.endReason === undefined ? 1 - progress : 1);
    }
  });
}

function drawWorker({ context, layout, scene }: Frame, worker: StackAttackWorker, feet: CellPoint): void {
  const cell = layout.cellSize;
  const left = layout.cellLeft(feet.x);
  const feetTop = layout.cellTop(feet.y);
  const headTop = layout.cellTop(feet.y + 1);
  context.fillStyle = workerColor(worker.slot);
  context.beginPath();
  context.arc(left + cell / 2, headTop + cell * 0.4, cell * 0.33, 0, Math.PI * 2);
  context.fill();
  context.fillRect(left + cell * 0.25, headTop + cell * 0.75, cell * 0.5, cell * 0.5);
  context.fillRect(left + cell * 0.3, feetTop + cell * 0.2, cell * 0.4, cell * 0.5);
  context.fillRect(left + cell * 0.2, feetTop + cell * 0.7, cell * 0.25, cell * 0.3);
  context.fillRect(left + cell * 0.55, feetTop + cell * 0.7, cell * 0.25, cell * 0.3);
  if (worker.slot !== scene.view.you) return;
  context.fillStyle = YOU_MARKER;
  context.beginPath();
  context.moveTo(left + cell * 0.3, headTop - cell * 0.3);
  context.lineTo(left + cell * 0.7, headTop - cell * 0.3);
  context.lineTo(left + cell * 0.5, headTop - cell * 0.05);
  context.closePath();
  context.fill();
}

function drawCrushedWorker(
  { context, layout }: Frame,
  worker: StackAttackWorker,
  feet: CellPoint,
  opacity: number,
): void {
  const cell = layout.cellSize;
  context.save();
  context.globalAlpha = opacity;
  context.fillStyle = workerColor(worker.slot);
  context.fillRect(
    layout.cellLeft(feet.x) + cell * 0.05,
    layout.cellTop(feet.y) + cell * 0.78,
    cell * 0.9,
    cell * 0.22,
  );
  context.restore();
}
