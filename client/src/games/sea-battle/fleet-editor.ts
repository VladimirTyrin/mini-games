import { type SeaBattleShip, SeaBattleOrientation } from "@/proto/minigames/v2/sea_battle_pb";

export const SIDE = 10;

/** A ship on the board: its bow (top or left end), length and direction. */
export interface EditorShip {
  x: number;
  y: number;
  length: number;
  horizontal: boolean;
}

export interface CellPoint {
  x: number;
  y: number;
}

/** Row-major index of a cell, `0..SIDE * SIDE`. */
export function cellIndex(cell: CellPoint): number {
  return cell.y * SIDE + cell.x;
}

function cellAt(index: number): CellPoint {
  return { x: index % SIDE, y: Math.floor(index / SIDE) };
}

function neighbours(index: number, withCorners: boolean): number[] {
  const { x, y } = cellAt(index);
  const found: number[] = [];
  for (let dy = -1; dy <= 1; dy += 1) {
    for (let dx = -1; dx <= 1; dx += 1) {
      if ((dx === 0 && dy === 0) || (!withCorners && dx !== 0 && dy !== 0)) continue;
      const nx = x + dx;
      const ny = y + dy;
      if (nx >= 0 && nx < SIDE && ny >= 0 && ny < SIDE) found.push(ny * SIDE + nx);
    }
  }
  return found;
}

export function shipCells(ship: EditorShip): CellPoint[] {
  return Array.from({ length: ship.length }, (_, step) =>
    ship.horizontal ? { x: ship.x + step, y: ship.y } : { x: ship.x, y: ship.y + step },
  );
}

/** Groups of filled cells joined by a side, each sorted from the top left. */
function groups(filled: ReadonlySet<number>): number[][] {
  const seen = new Set<number>();
  const found: number[][] = [];
  for (const start of [...filled].sort((a, b) => a - b)) {
    if (seen.has(start)) continue;
    const group: number[] = [];
    const queue = [start];
    seen.add(start);
    while (queue.length > 0) {
      const index = queue.pop() as number;
      group.push(index);
      for (const next of neighbours(index, false)) {
        if (filled.has(next) && !seen.has(next)) {
          seen.add(next);
          queue.push(next);
        }
      }
    }
    found.push(group.sort((a, b) => a - b));
  }
  return found;
}

/** A straight group as a ship; `null` for a bent one. */
function asShip(group: readonly number[]): EditorShip | null {
  const cells = group.map(cellAt);
  const first = cells[0];
  if (!first) return null;
  if (cells.every((cell) => cell.y === first.y)) return { ...first, length: cells.length, horizontal: true };
  if (cells.every((cell) => cell.x === first.x)) return { ...first, length: cells.length, horizontal: false };
  return null;
}

/** The ships the filled cells make (cells joined by a side are one ship). */
export function shipsOf(filled: ReadonlySet<number>): EditorShip[] {
  return groups(filled).flatMap((group) => asShip(group) ?? []);
}

/**
 * Whether the filled cells keep to the fleet's rules so far: every group straight and no longer
 * than the longest ship, no two groups touching at a corner, and for every length from two up no
 * more groups that long or longer than the fleet has ships. Lone decks are not counted: two of
 * them may still become one ship when the cell between them is filled. A hint for the player while
 * placing (whether the rest still fits is `roomForTheRest`); the core decides whether the finished
 * fleet is legal.
 */
export function canGrowInto(filled: ReadonlySet<number>, lengths: readonly number[]): boolean {
  const total = lengths.reduce((sum, length) => sum + length, 0);
  if (filled.size > total) return false;
  const all = groups(filled);
  const groupOf = new Map<number, number>();
  all.forEach((group, at) => group.forEach((index) => groupOf.set(index, at)));
  for (const [at, group] of all.entries()) {
    if (asShip(group) === null) return false;
    const touches = group.some((index) =>
      neighbours(index, true).some((near) => groupOf.has(near) && groupOf.get(near) !== at),
    );
    if (touches) return false;
  }
  const longest = Math.max(0, ...lengths);
  for (let length = 2; length <= longest + 1; length += 1) {
    const groupsThatLong = all.filter((group) => group.length >= length).length;
    const shipsThatLong = lengths.filter((ship) => ship >= length).length;
    if (groupsThatLong > shipsThatLong) return false;
  }
  return true;
}

/** Empty cells where one more deck keeps to the fleet's rules (see `canGrowInto`). */
export function allowedCells(filled: ReadonlySet<number>, lengths: readonly number[]): Set<number> {
  const allowed = new Set<number>();
  for (let index = 0; index < SIDE * SIDE; index += 1) {
    if (filled.has(index)) continue;
    if (canGrowInto(new Set([...filled, index]), lengths)) allowed.add(index);
  }
  return allowed;
}

/** Placements tried before the search for room gives up and assumes there is some. */
const ROOM_SEARCH_BUDGET = 20_000;

export type Room = "fits" | "full";

/**
 * Whether the ships not yet started still have room: each group on the board stands for the
 * shortest ship it can still become, and the others must fit, longest first, on cells that touch
 * nothing placed. A bounded search: when it runs out of tries it answers `fits`.
 */
export function roomForTheRest(filled: ReadonlySet<number>, lengths: readonly number[]): Room {
  const notStarted = [...lengths].sort((a, b) => a - b);
  for (const size of groups(filled)
    .map((group) => group.length)
    .sort((a, b) => b - a)) {
    const at = notStarted.findIndex((length) => length >= size);
    if (at >= 0) notStarted.splice(at, 1);
  }
  notStarted.reverse();
  const blocked = new Set<number>();
  for (const index of filled) {
    blocked.add(index);
    for (const near of neighbours(index, true)) blocked.add(near);
  }
  let budget = ROOM_SEARCH_BUDGET;
  const place = (next: number): boolean => {
    const length = notStarted[next];
    if (length === undefined) return true;
    for (let index = 0; index < SIDE * SIDE; index += 1) {
      for (const horizontal of length === 1 ? [true] : [true, false]) {
        budget -= 1;
        if (budget <= 0) return true;
        const ship = { ...cellAt(index), length, horizontal };
        const cells = shipCells(ship);
        const fits = cells.every((cell) => cell.x < SIDE && cell.y < SIDE && !blocked.has(cellIndex(cell)));
        if (!fits) continue;
        const added = cells
          .flatMap((cell) => [cellIndex(cell), ...neighbours(cellIndex(cell), true)])
          .filter((taken) => !blocked.has(taken));
        for (const taken of added) blocked.add(taken);
        const rest = place(next + 1);
        for (const taken of added) blocked.delete(taken);
        if (rest) return true;
      }
    }
    return false;
  };
  return place(0) ? "fits" : "full";
}

/** The fleet's ship lengths, longest first, each with whether a ship of it is on the board. */
export function shipsLeft(
  filled: ReadonlySet<number>,
  lengths: readonly number[],
): { length: number; isPlaced: boolean }[] {
  const placed = shipsOf(filled).map((ship) => ship.length);
  return [...lengths]
    .sort((a, b) => b - a)
    .map((length) => {
      const at = placed.indexOf(length);
      if (at >= 0) placed.splice(at, 1);
      return { length, isPlaced: at >= 0 };
    });
}

export function cellsOf(ships: readonly EditorShip[]): Set<number> {
  return new Set(ships.flatMap((ship) => shipCells(ship).map(cellIndex)));
}

export function fromWire(ship: SeaBattleShip): EditorShip {
  return {
    x: ship.bow?.x ?? 0,
    y: ship.bow?.y ?? 0,
    length: ship.length,
    horizontal: ship.orientation !== SeaBattleOrientation.VERTICAL,
  };
}

export function toWire(ship: EditorShip): {
  bow: CellPoint;
  length: number;
  orientation: SeaBattleOrientation;
} {
  return {
    bow: { x: ship.x, y: ship.y },
    length: ship.length,
    orientation: ship.horizontal ? SeaBattleOrientation.HORIZONTAL : SeaBattleOrientation.VERTICAL,
  };
}
