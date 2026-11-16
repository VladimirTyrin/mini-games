const BOX_FILL = "#ed8936";
const BOX_LIGHT = "#fbb26a";
const BOX_SHADE = "#c25a1c";
const BOX_EDGE = "#5f2a0e";
// Dark enough to read at a glance on the orange face (the old mid-orange marks blended in).
const BOX_MARK = "#4a1d07";

/** A crate with the marking of its pattern (0..8), drawn into a `size` square at (x, y). */
export function drawBox(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  size: number,
  patternId: number,
): void {
  const inset = Math.max(1, size * 0.05);
  const inner = size - 2 * inset;
  const bevel = Math.max(1, size * 0.08);
  context.fillStyle = BOX_SHADE;
  context.fillRect(x + inset, y + inset, inner, inner);
  context.fillStyle = BOX_LIGHT;
  context.fillRect(x + inset, y + inset, inner - bevel, inner - bevel);
  context.fillStyle = BOX_FILL;
  context.fillRect(x + inset + bevel, y + inset + bevel, inner - 2 * bevel, inner - 2 * bevel);
  context.strokeStyle = BOX_EDGE;
  context.lineWidth = Math.max(1, size * 0.06);
  context.strokeRect(x + inset, y + inset, inner, inner);
  context.fillStyle = BOX_MARK;
  context.strokeStyle = BOX_MARK;
  drawMarking(context, x, y, size, patternId);
}

function drawMarking(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  size: number,
  patternId: number,
): void {
  const dot = size * 0.14;
  const near = size * 0.22;
  const middle = size / 2 - dot / 2;
  const far = size - near - dot;
  const dots = (...cells: readonly (readonly [number, number])[]) => {
    for (const [left, top] of cells) context.fillRect(x + left, y + top, dot, dot);
  };
  switch (patternId) {
    case 0:
      dots([near, near], [far, near], [near, far], [far, far]);
      break;
    case 1:
      dots([middle, near], [middle, far]);
      break;
    case 2:
      dots([near, middle], [far, middle]);
      break;
    case 3:
      dots([middle, middle]);
      break;
    case 4:
      context.fillRect(x + near, y + near, size - 2 * near, dot);
      context.fillRect(x + near, y + far, size - 2 * near, dot);
      break;
    case 5:
      context.fillRect(x + near, y + near, dot, size - 2 * near);
      context.fillRect(x + far, y + near, dot, size - 2 * near);
      break;
    case 6:
      context.lineWidth = dot * 0.8;
      context.beginPath();
      context.moveTo(x + near, y + near);
      context.lineTo(x + size - near, y + size - near);
      context.moveTo(x + size - near, y + near);
      context.lineTo(x + near, y + size - near);
      context.stroke();
      break;
    case 7:
      dots(
        [near, near],
        [middle, near],
        [far, near],
        [near, middle],
        [far, middle],
        [near, far],
        [middle, far],
        [far, far],
      );
      break;
    default:
      context.fillRect(x + near, y + middle, size - 2 * near, dot);
      context.fillRect(x + middle, y + near, dot, size - 2 * near);
  }
}
