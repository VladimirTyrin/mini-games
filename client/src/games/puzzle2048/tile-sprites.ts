import type { Puzzle2048Cell, Puzzle2048View } from "@/proto/minigames/v2/puzzle2048_pb";

export interface BoardPosition {
  readonly row: number;
  readonly col: number;
}

/**
 * How a sprite enters the latest frame. Animation order: `slide` and `mergeSource` sprites move
 * first, then `merge` sprites pop and `spawn` sprites appear last.
 */
export type TileEntrance =
  | { readonly kind: "static" }
  | { readonly kind: "slide"; readonly from: BoardPosition }
  /** A tile consumed by a merge: slides into the merged tile's cell and disappears. */
  | { readonly kind: "mergeSource"; readonly from: BoardPosition }
  | { readonly kind: "merge" }
  | { readonly kind: "spawn" };

export interface TileSprite {
  /** Tile id; ids are never reused in a match, so merge sources never collide with live tiles. */
  readonly id: number;
  readonly value: number;
  readonly position: BoardPosition;
  readonly entrance: TileEntrance;
}

const STATIC: TileEntrance = { kind: "static" };

function toPosition(cell: Puzzle2048Cell): BoardPosition {
  return { row: cell.row, col: cell.col };
}

function slideFrom(from: Puzzle2048Cell | undefined, position: BoardPosition): TileEntrance {
  if (!from || (from.row === position.row && from.col === position.col)) return STATIC;
  return { kind: "slide", from: toPosition(from) };
}

/** Everything to draw for the view's latest frame, derived from tile ids and origins alone. */
export function tileSprites(view: Puzzle2048View): TileSprite[] {
  const sprites: TileSprite[] = [];
  for (const tile of view.tiles) {
    if (!tile.cell) continue;
    const position = toPosition(tile.cell);
    const origin = tile.origin;
    switch (origin.case) {
      case "slid":
        sprites.push({
          id: tile.id,
          value: tile.value,
          position,
          entrance: slideFrom(origin.value.from, position),
        });
        break;
      case "merged":
        for (const source of origin.value.sources) {
          if (!source.from) continue;
          // The view does not carry source values; both sources of a merge hold half of it.
          const sourceValue = tile.value / 2;
          sprites.push({
            id: source.id,
            value: sourceValue,
            position,
            entrance: { kind: "mergeSource", from: toPosition(source.from) },
          });
        }
        sprites.push({ id: tile.id, value: tile.value, position, entrance: { kind: "merge" } });
        break;
      case "spawned":
        sprites.push({ id: tile.id, value: tile.value, position, entrance: { kind: "spawn" } });
        break;
      default:
        sprites.push({ id: tile.id, value: tile.value, position, entrance: STATIC });
    }
  }
  return sprites;
}
