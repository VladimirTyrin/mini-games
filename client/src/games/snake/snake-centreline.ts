import type { CellPosition } from "./snake-geometry";

/** A point of the body's centre line, in cells. */
export interface CentrelinePoint {
  readonly x: number;
  readonly y: number;
  /** Direction of travel here (towards the head), in radians. */
  readonly angle: number;
  /** Distance along the line to the tail end, in cells. */
  readonly fromTail: number;
}

/** Largest corner radius: half a cell keeps a turn inside its cell. */
const CORNER_RADIUS = 0.5;
/** Spacing of the sampled points; fine enough that rounded corners look round. */
const STEP = 0.05;

function distance(from: CellPosition, to: CellPosition): number {
  return Math.hypot(to.x - from.x, to.y - from.y);
}

/**
 * The body's centre line from head to tail with rounded corners, as closely spaced points.
 * `points` runs head first (see `snakeBodyPath`). The head and tail segments shrink to nothing
 * as the snake moves, so a corner never rounds past them; two corners in a row share the
 * segment between them.
 */
export function sampleCentreline(points: readonly CellPosition[]): CentrelinePoint[] {
  const distinct = points.filter((point, index) => {
    const before = points[index - 1];
    return !before || distance(before, point) > 1e-3;
  });
  // Walked tail first, so every sample knows its distance from the tail as it is made.
  const tailFirst = [...distinct].reverse();
  const samples: { x: number; y: number }[] = [];
  const first = tailFirst[0];
  if (!first) return [];
  samples.push(first);
  let cursor: CellPosition = first;
  for (let index = 1; index < tailFirst.length; index++) {
    const corner = tailFirst[index];
    const after = tailFirst[index + 1];
    if (!corner) break;
    if (!after) {
      pushLine(samples, cursor, corner);
      cursor = corner;
      break;
    }
    const into = distance(tailFirst[index - 1] ?? cursor, corner) * (index === 1 ? 1 : 0.5);
    const out = distance(corner, after) * (index === tailFirst.length - 2 ? 1 : 0.5);
    const radius = Math.min(CORNER_RADIUS, into, out);
    const inX = Math.sign(corner.x - cursor.x);
    const inY = Math.sign(corner.y - cursor.y);
    const outX = Math.sign(after.x - corner.x);
    const outY = Math.sign(after.y - corner.y);
    const turns = inX !== outX || inY !== outY;
    if (!turns || radius <= 1e-3) {
      pushLine(samples, cursor, corner);
      cursor = corner;
      continue;
    }
    const start = { x: corner.x - inX * radius, y: corner.y - inY * radius };
    const end = { x: corner.x + outX * radius, y: corner.y + outY * radius };
    pushLine(samples, cursor, start);
    pushQuarterArc(samples, start, end, { x: start.x + outX * radius, y: start.y + outY * radius });
    cursor = end;
  }
  return withDistances(samples);
}

function pushLine(samples: { x: number; y: number }[], from: CellPosition, to: CellPosition): void {
  const steps = Math.max(1, Math.ceil(distance(from, to) / STEP));
  for (let step = 1; step <= steps; step++) {
    const t = step / steps;
    samples.push({ x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t });
  }
}

/** A grid turn is a right angle: a quarter circle around `centre` from `start` to `end`. */
function pushQuarterArc(
  samples: { x: number; y: number }[],
  start: CellPosition,
  end: CellPosition,
  centre: CellPosition,
): void {
  const radius = distance(centre, start);
  const from = Math.atan2(start.y - centre.y, start.x - centre.x);
  let sweep = Math.atan2(end.y - centre.y, end.x - centre.x) - from;
  if (sweep > Math.PI) sweep -= 2 * Math.PI;
  if (sweep < -Math.PI) sweep += 2 * Math.PI;
  const steps = Math.max(2, Math.ceil((Math.abs(sweep) * radius) / STEP));
  for (let step = 1; step <= steps; step++) {
    const angle = from + (sweep * step) / steps;
    samples.push({ x: centre.x + Math.cos(angle) * radius, y: centre.y + Math.sin(angle) * radius });
  }
}

function withDistances(tailFirst: readonly { x: number; y: number }[]): CentrelinePoint[] {
  const result: CentrelinePoint[] = [];
  let fromTail = 0;
  let previous: { x: number; y: number } | undefined;
  for (const [index, point] of tailFirst.entries()) {
    if (previous) fromTail += distance(previous, point);
    const toward = tailFirst[index + 1] ?? point;
    const behind = previous ?? point;
    const angle =
      toward === point
        ? Math.atan2(point.y - behind.y, point.x - behind.x)
        : Math.atan2(toward.y - point.y, toward.x - point.x);
    result.push({ x: point.x, y: point.y, angle, fromTail });
    previous = point;
  }
  return result.reverse();
}
