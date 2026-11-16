import type { CanvasSize } from "@/render/use-canvas";

/** Rail and trolleys above the row where cranes carry their boxes. */
export const CRANE_RAIL_ROWS = 0.6;
const CARRIED_BOX_ROWS = 1;
const FLOOR_ROWS = 0.35;

/**
 * Where game cells land on the canvas. Game rows count from the floor (y = 0) upwards; above the
 * field sits the row of boxes carried by cranes (y = fieldHeight) and above that the crane rail.
 */
export class StackAttackLayout {
  private constructor(
    readonly cellSize: number,
    readonly originX: number,
    readonly originY: number,
    readonly fieldWidth: number,
    readonly fieldHeight: number,
  ) {}

  /** Largest whole-pixel cells that fit, centred. */
  static fit(canvas: CanvasSize, fieldWidth: number, fieldHeight: number): StackAttackLayout {
    const rows = CRANE_RAIL_ROWS + CARRIED_BOX_ROWS + fieldHeight + FLOOR_ROWS;
    const fit = Math.min(canvas.width / fieldWidth, canvas.height / rows);
    const cellSize = fit >= 1 ? Math.floor(fit) : fit;
    return new StackAttackLayout(
      cellSize,
      Math.floor((canvas.width - cellSize * fieldWidth) / 2),
      Math.floor((canvas.height - cellSize * rows) / 2),
      fieldWidth,
      fieldHeight,
    );
  }

  get widthPx(): number {
    return this.fieldWidth * this.cellSize;
  }

  /** Top edge of the topmost field row (the ceiling). */
  get ceilingY(): number {
    return this.cellTop(this.fieldHeight - 1);
  }

  get floorY(): number {
    return this.cellTop(-1);
  }

  get bottomY(): number {
    return this.floorY + FLOOR_ROWS * this.cellSize;
  }

  cellLeft(x: number): number {
    return this.originX + x * this.cellSize;
  }

  /** Accepts fractional rows while something moves between cells. */
  cellTop(y: number): number {
    return this.originY + (CRANE_RAIL_ROWS + CARRIED_BOX_ROWS + this.fieldHeight - 1 - y) * this.cellSize;
  }
}
