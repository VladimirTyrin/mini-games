import {
  SnakeDeadSnakeBehavior,
  type SnakeSeatView,
  type SnakeView,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";
import type { CanvasSize } from "@/render/use-canvas";

import { type CellPosition, type FieldSize, type GridCell, sameCell, wrappedCopies } from "./snake-geometry";
import { DEAD_SNAKE_COLOR, type RgbColor, cssColor, snakeColor, tintedBySprite } from "./snake-palette";
import { type CentrelinePoint, sampleCentreline } from "./snake-centreline";
import { type Crash, crashesOf } from "./snake-crash";
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
/** The ring that points out the viewer's snake at the start fades out within these first ms. */
const YOU_RING_MS = 5_000;
const FOOD_COLOR = "#ef4444";
const YOU_RING_COLOR = "rgb(255 255 255 / 0.85)";
/** A snake flashes this colour in the tick it crashes, then turns grey. */
const CRASH_FLASH: RgbColor = { red: 248, green: 72, blue: 72 };
const CRASH_FILL = "#ef4444";
const CRASH_OUTLINE = "#450a0a";
const CRASH_CORE = "#fde68a";
const CRASH_BURST = "#fbbf24";

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
  const crashes = crashesOf(scene.view, scene.previous);
  // Under the snakes, so a dead one's face shows over its star; not clipped to the field, as a
  // crash into a wall is on its edge.
  for (const crash of crashes) drawCrash(frame, crash, "star");
  context.save();
  context.beginPath();
  context.rect(0, 0, fieldWidthPx, fieldHeightPx);
  context.clip();
  drawFood(frame);
  for (const seat of drawingOrder(scene.view)) drawSnake(frame, seat);
  context.restore();
  drawBorder(frame, fieldWidthPx, fieldHeightPx);
  for (const crash of crashes) drawCrash(frame, crash, "burst");
  context.restore();
}

/** The checkered ground as a pattern of two by two cells, made once per cell size. */
let groundTile: { cellSize: number; pattern: CanvasPattern } | null = null;

function groundPattern(context: CanvasRenderingContext2D, cellSize: number): CanvasPattern | null {
  if (groundTile?.cellSize === cellSize) return groundTile.pattern;
  const tile = document.createElement("canvas");
  tile.width = tile.height = Math.max(1, Math.round(cellSize * 2));
  const tileContext = tile.getContext("2d");
  if (!tileContext) return null;
  tileContext.fillStyle = GROUND_LIGHT;
  tileContext.fillRect(0, 0, tile.width, tile.height);
  tileContext.fillStyle = GROUND_DARK;
  tileContext.fillRect(0, 0, cellSize, cellSize);
  tileContext.fillRect(cellSize, cellSize, cellSize, cellSize);
  const pattern = context.createPattern(tile, "repeat");
  if (!pattern) return null;
  groundTile = { cellSize, pattern };
  return pattern;
}

// One fill with a cached pattern: drawing the cells one by one cost hundreds of rectangles a frame.
function drawGround({ context, field, cellSize }: Frame): void {
  context.fillStyle = groundPattern(context, cellSize) ?? GROUND_LIGHT;
  context.fillRect(0, 0, field.width * cellSize, field.height * cellSize);
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
  const color = seat.alive
    ? snakeColor(seat.slot)
    : previousSeat?.alive
      ? mixed(CRASH_FLASH, DEAD_SNAKE_COLOR, scene.progress)
      : DEAD_SNAKE_COLOR;
  // Sampled once: the copies across a wrap-around border are the same line, shifted.
  const line = sampleCentreline(path.points);
  context.save();
  context.globalAlpha = opacity;
  // The centre line may run past a wrap-around border; shifted copies show the part that
  // re-enters on the other side (the field clip hides the rest).
  for (const shift of borderShifts(path.points, field)) {
    context.save();
    context.translate(shift.x * frame.cellSize, shift.y * frame.cellSize);
    drawBody(frame, line, color);
    if (!seat.alive) drawDeadTongue(frame, path);
    drawHead(frame, path, color);
    if (!seat.alive) drawDeadEyes(frame, path, color);
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

/** Head drawn as a plain disc until the sprite sheet has loaded. */
const FALLBACK_HEAD_RADIUS = 0.4;
/** The tail narrows over this length (cells) to TAIL_TIP of the tube's width. */
const TAIL_TAPER = 2.4;
const TAIL_TIP = 0.32;
/** The taper outline starts this far inside the full-width stroke, so no seam shows. */
const TAPER_OVERLAP = 0.1;
/** Spots along the back, this far apart (cells). A whole fraction of a cell, so the pattern
 * lines up again after every tick's one-cell step. */
const SPOT_SPACING = 0.5;
/** No spots this close to the head: the head sprite covers it. */
const SPOT_HEAD_CLEARANCE = 0.55;

/** Width of the tube at `fromTail` cells from the tail end, as a share of the full width. */
function bodyWidthAt(fromTail: number, taper: number): number {
  if (fromTail >= taper) return 1;
  // Quarter sine: the body keeps its girth for most of the taper and narrows near the end.
  return TAIL_TIP + (1 - TAIL_TIP) * Math.sin(((fromTail / taper) * Math.PI) / 2);
}

/**
 * The v1 tube along the centre line (outline, side bands, centre, recoloured for the seat),
 * narrowing towards the tail, with spots that sit on the body itself: they are counted from the
 * tail end, so they slide forward with the snake and show it moving.
 */
function drawBody(frame: Frame, line: readonly CentrelinePoint[], color: RgbColor): void {
  const { context, cellSize } = frame;
  const length = line[0]?.fromTail ?? 0;
  if (line.length < 2 || length <= 0) return;
  const taper = Math.min(TAIL_TAPER, length * 0.4);
  // The full-width part is one stroke; the taper is a filled outline per layer, so the three
  // bands narrow together and the edge stays smooth.
  const full = strokeVertices(line.filter((point) => point.fromTail >= taper));
  const tail = line.filter((point) => point.fromTail <= taper + TAPER_OVERLAP);
  const layers = [
    { width: TUBE.width, sheetColor: TUBE.outline },
    { width: TUBE.width - 2 * TUBE.outlineWidth, sheetColor: TUBE.side },
    { width: TUBE.centreWidth, sheetColor: TUBE.centre },
  ];
  for (const layer of layers) {
    const style = cssColor(tintedBySprite(color, layer.sheetColor));
    if (full.length >= 2) {
      context.beginPath();
      for (const [index, point] of full.entries()) {
        if (index === 0) context.moveTo(point.x * cellSize, point.y * cellSize);
        else context.lineTo(point.x * cellSize, point.y * cellSize);
      }
      // Butt at the tail side, where the taper outline continues it; the head covers the
      // other end.
      context.lineCap = "butt";
      context.lineJoin = "round";
      context.lineWidth = layer.width * cellSize;
      context.strokeStyle = style;
      context.stroke();
    }
    fillTaper(context, tail, taper, layer.width * cellSize, cellSize, style);
  }
  drawSpots(frame, line, length, taper, color);
}

/**
 * The points a stroke needs: the corners' arcs, and only the ends of each straight run. The line
 * is sampled finely everywhere (spots and the taper read it), but a straight run's inner points
 * add nothing to a stroke and cost the canvas thousands of vertices a frame.
 */
export function strokeVertices(line: readonly CentrelinePoint[]): CentrelinePoint[] {
  const sameDirection = (a: CentrelinePoint | undefined, b: CentrelinePoint | undefined) =>
    a !== undefined && b !== undefined && Math.abs(a.angle - b.angle) < 1e-9;
  return line.filter(
    (point, index) => !sameDirection(line[index - 1], point) || !sameDirection(point, line[index + 1]),
  );
}

/**
 * The narrowing end of one tube layer as a filled outline: both edges offset from the centre
 * line by half the width there, closed by a round tip.
 */
function fillTaper(
  context: CanvasRenderingContext2D,
  tail: readonly CentrelinePoint[],
  taper: number,
  width: number,
  cellSize: number,
  style: string,
): void {
  if (tail.length < 2) return;
  const edge = (point: CentrelinePoint, side: 1 | -1) => {
    const half = (width * bodyWidthAt(point.fromTail, taper)) / 2;
    return {
      x: point.x * cellSize - Math.sin(point.angle) * half * side,
      y: point.y * cellSize + Math.cos(point.angle) * half * side,
    };
  };
  context.beginPath();
  for (const [index, point] of tail.entries()) {
    const left = edge(point, 1);
    if (index === 0) context.moveTo(left.x, left.y);
    else context.lineTo(left.x, left.y);
  }
  const tip = tail.at(-1);
  if (tip) {
    const radius = (width * bodyWidthAt(0, taper)) / 2;
    // Around the far side of the tail end, from the left edge to the right one.
    context.arc(tip.x * cellSize, tip.y * cellSize, radius, tip.angle + Math.PI / 2, tip.angle - Math.PI / 2);
  }
  for (const point of [...tail].reverse()) {
    const right = edge(point, -1);
    context.lineTo(right.x, right.y);
  }
  context.closePath();
  context.fillStyle = style;
  context.fill();
}

function drawSpots(
  frame: Frame,
  line: readonly CentrelinePoint[],
  length: number,
  taper: number,
  color: RgbColor,
): void {
  const { context, cellSize } = frame;
  context.save();
  context.fillStyle = cssColor(tintedBySprite(color, TUBE.outline));
  context.globalAlpha *= 0.7;
  // `line` runs head first, so its distances from the tail fall; walk them with one cursor. All
  // spots go into one path: a fill per spot was dozens of draw calls a frame for a long snake.
  context.beginPath();
  let index = line.length - 1;
  for (let at = SPOT_SPACING; at < length - SPOT_HEAD_CLEARANCE; at += SPOT_SPACING) {
    while (index > 0 && (line[index - 1]?.fromTail ?? Infinity) <= at) index--;
    const point = line[index];
    if (!point) break;
    const size = bodyWidthAt(at, taper) * cellSize;
    context.moveTo(
      point.x * cellSize + size * 0.1 * Math.cos(point.angle),
      point.y * cellSize + size * 0.1 * Math.sin(point.angle),
    );
    context.ellipse(
      point.x * cellSize,
      point.y * cellSize,
      size * 0.1,
      size * 0.065,
      point.angle,
      0,
      Math.PI * 2,
    );
  }
  context.fill();
  context.restore();
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

function mixed(from: RgbColor, to: RgbColor, amount: number): RgbColor {
  const mix = (a: number, b: number) => Math.round(a + (b - a) * amount);
  return { red: mix(from.red, to.red), green: mix(from.green, to.green), blue: mix(from.blue, to.blue) };
}

/** A crash shows as long as one of its snakes does (all of them at the end of the match). */
function crashOpacity(crash: Crash, scene: SnakeScene): number {
  if (crash.fresh) return 1;
  return Math.max(
    0,
    ...crash.slots.map((slot) => {
      const seat = scene.view.snakes[slot];
      return seat ? snakeOpacity(seat, scene.previous?.snakes[slot], scene) : 0;
    }),
  );
}

/**
 * Where snakes crashed: a star that stays with the dead snakes, larger where two of them met
 * head-on, and over everything a burst in the tick it happens.
 */
function drawCrash(frame: Frame, crash: Crash, layer: "star" | "burst"): void {
  const { context, cellSize, scene, field } = frame;
  const opacity = crashOpacity(crash, scene);
  if (opacity <= 0) return;
  const size = (crash.headOn ? 0.62 : 0.46) * cellSize;
  // The star pops in over the first half of the tick, the burst runs over the whole of it.
  const grow = crash.fresh ? Math.max(0, easeOutBack(Math.min(1, scene.progress * 2))) : 1;
  const positions =
    scene.view.wallCollisionMode === SnakeWallCollisionMode.WRAP_AROUND
      ? wrappedCopies({ x: crash.at.x - 0.5, y: crash.at.y - 0.5 }, field).map((copy) => ({
          x: copy.x + 0.5,
          y: copy.y + 0.5,
        }))
      : [crash.at];
  for (const position of positions) {
    context.save();
    context.globalAlpha = opacity;
    context.translate(position.x * cellSize, position.y * cellSize);
    if (layer === "star") drawStar(context, size * grow, Math.max(1, cellSize * 0.06));
    else if (crash.fresh && scene.progress < 1) drawBurst(context, size, scene.progress);
    context.restore();
  }
}

function drawBurst(context: CanvasRenderingContext2D, size: number, progress: number): void {
  const reach = size * (1 + 1.8 * (1 - (1 - progress) ** 3));
  context.save();
  context.globalAlpha *= 1 - progress;
  context.strokeStyle = CRASH_BURST;
  context.lineWidth = Math.max(1.5, size * 0.14);
  context.lineCap = "round";
  context.beginPath();
  context.arc(0, 0, reach, 0, Math.PI * 2);
  context.stroke();
  for (let ray = 0; ray < 8; ray++) {
    const angle = (ray / 8) * Math.PI * 2 + Math.PI / 8;
    context.moveTo(Math.cos(angle) * reach * 0.55, Math.sin(angle) * reach * 0.55);
    context.lineTo(Math.cos(angle) * reach * 0.9, Math.sin(angle) * reach * 0.9);
  }
  context.stroke();
  context.restore();
}

function drawStar(context: CanvasRenderingContext2D, radius: number, outline: number): void {
  if (radius <= 0) return;
  const points = 8;
  context.beginPath();
  for (let vertex = 0; vertex < points * 2; vertex++) {
    const reach = vertex % 2 === 0 ? radius : radius * 0.48;
    const angle = (vertex / (points * 2)) * Math.PI * 2 - Math.PI / 2;
    const x = Math.cos(angle) * reach;
    const y = Math.sin(angle) * reach;
    if (vertex === 0) context.moveTo(x, y);
    else context.lineTo(x, y);
  }
  context.closePath();
  context.fillStyle = CRASH_FILL;
  context.fill();
  context.lineWidth = outline;
  context.strokeStyle = CRASH_OUTLINE;
  context.stroke();
  context.beginPath();
  context.arc(0, 0, radius * 0.28, 0, Math.PI * 2);
  context.fillStyle = CRASH_CORE;
  context.fill();
}

// A dead snake's face, in head-local units of a cell (facing right, centred on the head): where
// the head sprite has its eyes and its snout.
const DEAD_EYES: readonly CellPosition[] = [
  { x: 0.1, y: -0.18 },
  { x: 0.1, y: 0.17 },
];
const DEAD_EYE_SIZE = 0.09;
const DEAD_EYE_CROSS = "#111827";
const TONGUE = "#f472b6";
const TONGUE_OUTLINE = "#831843";

/** Turns the context to the head: origin on it, x along its heading, one unit per cell. */
function inHeadFrame(
  frame: Frame,
  path: SnakeBodyPath,
  draw: (context: CanvasRenderingContext2D) => void,
): void {
  const head = path.points[0];
  if (!head) return;
  const { context, cellSize } = frame;
  context.save();
  context.translate(head.x * cellSize, head.y * cellSize);
  context.rotate(path.heading);
  context.scale(cellSize, cellSize);
  draw(context);
  context.restore();
}

/**
 * Lolls out of the corner of the mouth, to the side: in front of the head sits the crash's star.
 * Drawn before the head, so its root hides under it.
 */
function drawDeadTongue(frame: Frame, path: SnakeBodyPath): void {
  inHeadFrame(frame, path, (context) => {
    context.translate(0.3, 0.12);
    context.rotate(1.05);
    context.beginPath();
    context.roundRect(0, -0.12, 0.56, 0.24, 0.12);
    context.fillStyle = TONGUE;
    context.fill();
    context.lineWidth = 0.03;
    context.strokeStyle = TONGUE_OUTLINE;
    context.stroke();
    context.beginPath();
    context.moveTo(0.18, 0);
    context.lineTo(0.42, 0);
    context.lineWidth = 0.025;
    context.lineCap = "round";
    context.stroke();
  });
}

/** Crosses over the eyes, which are painted over in the head's colour first. */
function drawDeadEyes(frame: Frame, path: SnakeBodyPath, color: RgbColor): void {
  inHeadFrame(frame, path, (context) => {
    for (const eye of DEAD_EYES) {
      context.beginPath();
      context.arc(eye.x, eye.y, DEAD_EYE_SIZE * 1.15, 0, Math.PI * 2);
      context.fillStyle = cssColor(color);
      context.fill();
      context.beginPath();
      context.moveTo(eye.x - DEAD_EYE_SIZE, eye.y - DEAD_EYE_SIZE);
      context.lineTo(eye.x + DEAD_EYE_SIZE, eye.y + DEAD_EYE_SIZE);
      context.moveTo(eye.x + DEAD_EYE_SIZE, eye.y - DEAD_EYE_SIZE);
      context.lineTo(eye.x - DEAD_EYE_SIZE, eye.y + DEAD_EYE_SIZE);
      context.lineWidth = 0.045;
      context.lineCap = "round";
      context.strokeStyle = DEAD_EYE_CROSS;
      context.stroke();
    }
  });
}
