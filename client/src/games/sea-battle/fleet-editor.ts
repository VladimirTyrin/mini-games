import { type SeaBattleShip, SeaBattleOrientation } from "@/proto/minigames/v2/sea_battle_pb";

export const SIDE = 10;

/** A ship being placed: its bow (top or left end), length and direction. */
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

export function shipCells(ship: EditorShip): CellPoint[] {
  return Array.from({ length: ship.length }, (_, step) =>
    ship.horizontal ? { x: ship.x + step, y: ship.y } : { x: ship.x, y: ship.y + step },
  );
}

/** The ship with its bow at (x, y), shifted back onto the board where it would stick out. */
export function placed(ship: EditorShip, x: number, y: number): EditorShip {
  const maxX = SIDE - (ship.horizontal ? ship.length : 1);
  const maxY = SIDE - (ship.horizontal ? 1 : ship.length);
  return { ...ship, x: clamp(x, 0, maxX), y: clamp(y, 0, maxY) };
}

/** Turned about its bow, kept on the board. */
export function rotated(ship: EditorShip): EditorShip {
  return placed({ ...ship, horizontal: !ship.horizontal }, ship.x, ship.y);
}

/**
 * Which ships stand on or right next to another, to draw them as misplaced while the player
 * arranges the fleet. The core still decides whether the fleet is legal.
 */
export function crowdedShips(ships: readonly EditorShip[]): Set<number> {
  const crowded = new Set<number>();
  ships.forEach((ship, index) => {
    ships.forEach((other, otherIndex) => {
      if (otherIndex <= index) return;
      const near = shipCells(ship).some((cell) =>
        shipCells(other).some(
          (otherCell) => Math.abs(cell.x - otherCell.x) <= 1 && Math.abs(cell.y - otherCell.y) <= 1,
        ),
      );
      if (near) {
        crowded.add(index);
        crowded.add(otherIndex);
      }
    });
  });
  return crowded;
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

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}
