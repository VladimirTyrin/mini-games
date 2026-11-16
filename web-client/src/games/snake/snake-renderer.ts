import {
  SnakeDeadSnakeBehavior,
  type SnakeSeatView,
  type SnakeView,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";
import type { CanvasSize } from "@/render/use-canvas";

import { type CellPosition, type FieldSize, type GridCell, sameCell, wrappedCopies } from "./snake-geometry";
import { DEAD_SNAKE_COLOR, type RgbColor, cssColor, snakeColor } from "./snake-palette";
import { poseSnake, type SpritePlacement } from "./snake-pose";
import type { SnakeSpriteSheet } from "./snake-sprite-sheet";
import type { SnakeSpriteName } from "./snake-sprites";

export interface SnakeScene {
  readonly view: SnakeView;
  /** The view one tick earlier; rendering runs from it to `view` as `progress` goes 0..1. */
  readonly previous: SnakeView | null;
  readonly progress: number;
  /** `null` until the sheet loads (plain shapes are drawn meanwhile). */
  readonly sprites: SnakeSpriteSheet | null;
}

export interface BoardLayout {
  readonly cellSize: number;
  readonly originX: number;
  readonly originY: number;
}

const GROUND_LIGHT = "#8fe88f";
const GROUND_DARK = "#84dc84";
const BORDER_COLOR = "#1f5130";
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
  const size = cellSize * scale;
  const x = (cell.x + 0.5) * cellSize - size / 2;
  const y = (cell.y + 0.5) * cellSize - size / 2;
  if (scene.sprites) {
    scene.sprites.draw(context, "apple", null, x, y, size);
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
  const pose = poseSnake(seat.body, seat.direction, previousBody, field, scene.progress);
  if (!pose) return;
  const color = seat.alive ? snakeColor(seat.slot) : DEAD_SNAKE_COLOR;
  context.save();
  context.globalAlpha = opacity;
  for (const placement of pose.placements) drawPlacement(frame, placement, color);
  if (seat.alive && seat.slot === scene.view.you) drawYouRing(frame, pose.head);
  context.restore();
}

function drawPlacement(frame: Frame, placement: SpritePlacement, color: RgbColor): void {
  const { context, cellSize } = frame;
  if (placement.clipTo) {
    context.save();
    context.beginPath();
    context.rect(placement.clipTo.x * cellSize, placement.clipTo.y * cellSize, cellSize, cellSize);
    context.clip();
  }
  for (const copy of wrappedCopies(placement.at, frame.field))
    drawSprite(frame, placement.sprite, color, copy);
  if (placement.clipTo) context.restore();
}

function drawSprite(frame: Frame, sprite: SnakeSpriteName, color: RgbColor, at: CellPosition): void {
  const { context, scene, cellSize } = frame;
  if (scene.sprites) {
    scene.sprites.draw(context, sprite, color, at.x * cellSize, at.y * cellSize, cellSize);
    return;
  }
  const inset = cellSize * 0.1;
  context.fillStyle = cssColor(color);
  context.fillRect(
    at.x * cellSize + inset,
    at.y * cellSize + inset,
    cellSize - 2 * inset,
    cellSize - 2 * inset,
  );
}

function drawYouRing(frame: Frame, head: CellPosition): void {
  const { context, cellSize } = frame;
  context.strokeStyle = YOU_RING_COLOR;
  context.lineWidth = Math.max(1.5, cellSize * 0.08);
  for (const copy of wrappedCopies(head, frame.field)) {
    context.beginPath();
    context.arc((copy.x + 0.5) * cellSize, (copy.y + 0.5) * cellSize, cellSize * 0.72, 0, Math.PI * 2);
    context.stroke();
  }
}
