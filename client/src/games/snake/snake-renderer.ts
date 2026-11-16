import {
  SnakeDeadSnakeBehavior,
  type SnakeSeatView,
  type SnakeView,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";
import type { CanvasSize } from "@/render/use-canvas";

import { type CellPosition, type FieldSize, type GridCell, sameCell, wrappedCopies } from "./snake-geometry";
import { DEAD_SNAKE_COLOR, type RgbColor, cssColor, snakeColor, tintedBySprite } from "./snake-palette";
import { type SnakeBodyPath, snakeBodyPath } from "./snake-path";
import type { SnakeSpriteSheet } from "./snake-sprite-sheet";
import { TUBE } from "./snake-sprites";

export interface SnakeScene {
  readonly view: SnakeView;
  /** The view one tick earlier; rendering runs from it to `view` as `progress` goes 0..1. */
  readonly previous: SnakeView | null;
  readonly progress: number;
  /** Apple and head sprites; `null` until they load (plain shapes are drawn meanwhile). */
  readonly sprites: SnakeSpriteSheet | null;
}

export interface BoardLayout {
  readonly cellSize: number;
  readonly originX: number;
  readonly originY: number;
}

// Dark grass in the app's palette: bright seat colours and the red apple stand out on it.
const GROUND_LIGHT = "#1d3527";
const GROUND_DARK = "#192e22";
const BORDER_COLOR = "#3f8a5a";
/** The ring that points out the viewer's snake fades out over the first seconds of a match. */
const YOU_RING_MS = 3_000;
const FOOD_COLOR = "#ef4444";
const YOU_RING_COLOR = "rgb(255 255 255 / 0.85)";

/** Largest whole-pixel cells that fit, centred. */
export function fitField(canvas: CanvasSize, field: FieldSize): BoardLayout {
  const fit = Math.min(canvas.width / field.width, canvas.height / field.height);
  const cellSize = fit >= 1 ? Math.floor(fit) : fit;
  return {
    cellSize,
    originX: Math.floor((canvas.width - cellSize * field.width) / 2),
    originY: Math.floor((canvas.height - cellSize * field.height) / 2),
  };
}

interface Frame {
  readonly context: CanvasRenderingContext2D;
  readonly scene: SnakeScene;
  readonly field: FieldSize;
  readonly cellSize: number;
}

export function drawSnakeScene(
  context: CanvasRenderingContext2D,
  canvas: CanvasSize,
  scene: SnakeScene,
): void {
  context.clearRect(0, 0, canvas.width, canvas.height);
  const field = { width: scene.view.fieldWidth, height: scene.view.fieldHeight };
  if (field.width <= 0 || field.height <= 0) return;
  const layout = fitField(canvas, field);
  const frame: Frame = { context, scene, field, cellSize: layout.cellSize };
  const fieldWidthPx = field.width * layout.cellSize;
  const fieldHeightPx = field.height * layout.cellSize;

  context.save();
  context.translate(layout.originX, layout.originY);
  drawGround(frame);
  context.save();
  context.beginPath();
  context.rect(0, 0, fieldWidthPx, fieldHeightPx);
  context.clip();
  drawFood(frame);
  for (const seat of drawingOrder(scene.view)) drawSnake(frame, seat);
  context.restore();
  drawBorder(frame, fieldWidthPx, fieldHeightPx);
  context.restore();
}

function drawGround({ context, field, cellSize }: Frame): void {
  context.fillStyle = GROUND_LIGHT;
  context.fillRect(0, 0, field.width * cellSize, field.height * cellSize);
  context.fillStyle = GROUND_DARK;
  context.beginPath();
  for (let y = 0; y < field.height; y++)
    for (let x = y % 2; x < field.width; x += 2) context.rect(x * cellSize, y * cellSize, cellSize, cellSize);
  context.fill();
}

function drawBorder({ context, scene }: Frame, width: number, height: number): void {
  context.strokeStyle = BORDER_COLOR;
  context.lineWidth = 2;
  // Dashed edges tell that snakes pass through them.
  context.setLineDash(scene.view.wallCollisionMode === SnakeWallCollisionMode.WRAP_AROUND ? [6, 6] : []);
  context.strokeRect(-1, -1, width + 2, height + 2);
  context.setLineDash([]);
}

function drawFood(frame: Frame): void {
  const { view, previous, progress } = frame.scene;
  for (const food of view.food) {
    const spawned = previous !== null && !previous.food.some((earlier) => sameCell(earlier, food));
    drawApple(frame, food, spawned ? easeOutBack(progress) : 1);
  }
  // Eaten food stays until the head that eats it has slid over it.
  for (const food of previous?.food ?? []) {
    if (!view.food.some((remaining) => sameCell(remaining, food))) drawApple(frame, food, 1);
  }
}

function easeOutBack(progress: number): number {
  const overshoot = 1.7;
  const t = progress - 1;
  return 1 + (overshoot + 1) * t * t * t + overshoot * t * t;
}

function drawApple({ context, scene, cellSize }: Frame, cell: GridCell, scale: number): void {
  // The pop-in easing starts a hair below zero; a negative size would make arc() throw.
  const size = cellSize * Math.max(0, scale);
  const x = (cell.x + 0.5) * cellSize - size / 2;
  const y = (cell.y + 0.5) * cellSize - size / 2;
  if (scene.sprites) {
    scene.sprites.drawApple(context, x, y, size);
    return;
  }
  context.fillStyle = FOOD_COLOR;
  context.beginPath();
  context.arc(x + size / 2, y + size / 2, size * 0.4, 0, Math.PI * 2);
  context.fill();
}

/** Dead snakes underneath, the viewer's own snake on top. */
function drawingOrder(view: SnakeView): SnakeSeatView[] {
  const rank = (seat: SnakeSeatView) => (!seat.alive ? 0 : seat.slot === view.you ? 2 : 1);
  return [...view.snakes].sort((first, second) => rank(first) - rank(second));
}

function snakeOpacity(
  seat: SnakeSeatView,
  previousSeat: SnakeSeatView | undefined,
  scene: SnakeScene,
): number {
  if (seat.alive || scene.view.deadSnakeBehavior === SnakeDeadSnakeBehavior.STAY_ON_FIELD) return 1;
  // A disappearing snake fades out during the tick it died in; the one that ended the match stays.
  if (!previousSeat?.alive) return 0;
  return scene.view.end ? 1 : 1 - scene.progress;
}

function drawSnake(frame: Frame, seat: SnakeSeatView): void {
  const { context, scene, field } = frame;
  const previousSeat = scene.previous?.snakes[seat.slot];
  const opacity = snakeOpacity(seat, previousSeat, scene);
  if (opacity <= 0) return;
  const previousBody = seat.alive && previousSeat ? previousSeat.body : null;
  const path = snakeBodyPath(seat.body, seat.direction, previousBody, field, scene.progress);
  if (!path) return;
  const color = seat.alive ? snakeColor(seat.slot) : DEAD_SNAKE_COLOR;
  context.save();
  context.globalAlpha = opacity;
  // The centre line may run past a wrap-around border; shifted copies show the part that
  // re-enters on the other side (the field clip hides the rest).
  for (const shift of borderShifts(path.points, field)) {
    context.save();
    context.translate(shift.x * frame.cellSize, shift.y * frame.cellSize);
    drawBody(frame, path.points, color);
    drawHead(frame, path, color);
    context.restore();
  }
  if (seat.alive && seat.slot === scene.view.you) drawYouRing(frame, path.points[0]);
  context.restore();
}

/** Offsets (in cells) of the copies of a line that overlap the field. */
function borderShifts(points: readonly CellPosition[], field: FieldSize): CellPosition[] {
  const xs = points.map((point) => point.x);
  const ys = points.map((point) => point.y);
  const minX = Math.min(...xs) - 1;
  const maxX = Math.max(...xs) + 1;
  const minY = Math.min(...ys) - 1;
  const maxY = Math.max(...ys) + 1;
  const shifts: CellPosition[] = [];
  for (const x of [-field.width, 0, field.width]) {
    for (const y of [-field.height, 0, field.height]) {
      if (maxX + x > 0 && minX + x < field.width && maxY + y > 0 && minY + y < field.height)
        shifts.push({ x, y });
    }
  }
  return shifts;
}

/** Largest corner radius: half a cell keeps a turn inside its cell. */
const CORNER_RADIUS = 0.5;
/** Head drawn as a plain disc until the sprite sheet has loaded. */
const FALLBACK_HEAD_RADIUS = 0.4;

function distance(from: CellPosition, to: CellPosition): number {
  return Math.hypot(to.x - from.x, to.y - from.y);
}

/** The centre line with rounded corners, in pixels. */
function traceLine(
  context: CanvasRenderingContext2D,
  points: readonly CellPosition[],
  cellSize: number,
): void {
  const distinct = points.filter((point, index) => {
    const before = points[index - 1];
    return !before || distance(before, point) > 1e-3;
  });
  const [first] = distinct;
  if (!first) return;
  context.beginPath();
  context.moveTo(first.x * cellSize, first.y * cellSize);
  for (let index = 1; index < distinct.length - 1; index++) {
    const before = distinct[index - 1];
    const corner = distinct[index];
    const after = distinct[index + 1];
    if (!before || !corner || !after) continue;
    // The head and tail segments shrink to nothing, so a corner never rounds past them; two
    // corners in a row share the segment between them.
    const into = distance(before, corner) * (index === 1 ? 1 : 0.5);
    const out = distance(corner, after) * (index === distinct.length - 2 ? 1 : 0.5);
    const radius = Math.min(CORNER_RADIUS, into, out) * cellSize;
    context.arcTo(corner.x * cellSize, corner.y * cellSize, after.x * cellSize, after.y * cellSize, radius);
  }
  const last = distinct.at(-1) ?? first;
  context.lineTo(last.x * cellSize, last.y * cellSize);
}

/** The v1 tube along the centre line: outline, side bands and centre, recoloured for the seat. */
function drawBody(frame: Frame, points: readonly CellPosition[], color: RgbColor): void {
  if (points.length < 2) return;
  const { context, cellSize } = frame;
  traceLine(context, points, cellSize);
  context.lineCap = "round";
  context.lineJoin = "round";
  const layers = [
    { width: TUBE.width, sheetColor: TUBE.outline },
    { width: TUBE.width - 2 * TUBE.outlineWidth, sheetColor: TUBE.side },
    { width: TUBE.centreWidth, sheetColor: TUBE.centre },
  ];
  for (const layer of layers) {
    context.strokeStyle = cssColor(tintedBySprite(color, layer.sheetColor));
    context.lineWidth = layer.width * cellSize;
    context.stroke();
  }
}

function drawHead(frame: Frame, path: SnakeBodyPath, color: RgbColor): void {
  const { context, cellSize, scene } = frame;
  const head = path.points[0];
  if (!head) return;
  const x = head.x * cellSize;
  const y = head.y * cellSize;
  if (scene.sprites) {
    scene.sprites.drawHead(context, color, x, y, cellSize, path.heading);
    return;
  }
  context.beginPath();
  context.arc(x, y, FALLBACK_HEAD_RADIUS * cellSize, 0, Math.PI * 2);
  context.fillStyle = cssColor(color);
  context.fill();
}

function drawYouRing(frame: Frame, head: CellPosition | undefined): void {
  if (!head) return;
  const { context, cellSize, scene } = frame;
  const elapsedMs = (Number(scene.view.tick) - 1 + scene.progress) * scene.view.tickIntervalMs;
  const visibility = 1 - elapsedMs / YOU_RING_MS;
  if (visibility <= 0) return;
  context.globalAlpha *= Math.min(1, visibility);
  context.strokeStyle = YOU_RING_COLOR;
  context.lineWidth = Math.max(1.5, cellSize * 0.08);
  for (const copy of wrappedCopies({ x: head.x - 0.5, y: head.y - 0.5 }, frame.field)) {
    context.beginPath();
    context.arc((copy.x + 0.5) * cellSize, (copy.y + 0.5) * cellSize, cellSize * 0.72, 0, Math.PI * 2);
    context.stroke();
  }
}
