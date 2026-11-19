import { cssColor, type RgbColor, tintedBySprite } from "./snake-palette";
import { APPLE_CELL, HEAD_FACING_RIGHT_CELL, SPRITE_PIXELS } from "./snake-sprites";

interface SheetCell {
  readonly column: number;
  readonly row: number;
}

interface SpriteCanvas {
  readonly canvas: HTMLCanvasElement;
  readonly context: CanvasRenderingContext2D;
}

/**
 * One cell of `source` on a canvas of its own. Drawn straight from the sheet, a sprite scaled up
 * past its 64 px blends in the edge of its neighbour there (filtering samples beyond the source
 * rectangle): the head showed a light line of the next row's neck along one side.
 */
function cutOut(
  source: CanvasImageSource,
  cell: SheetCell,
  settings?: CanvasRenderingContext2DSettings,
): SpriteCanvas | null {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = SPRITE_PIXELS;
  const context = canvas.getContext("2d", settings);
  if (!context) return null;
  context.drawImage(
    source,
    cell.column * SPRITE_PIXELS,
    cell.row * SPRITE_PIXELS,
    SPRITE_PIXELS,
    SPRITE_PIXELS,
    0,
    0,
    SPRITE_PIXELS,
    SPRITE_PIXELS,
  );
  return { canvas, context };
}

/**
 * The v1 sprites: the apple as drawn, and the head recoloured per seat (one copy per colour,
 * built on first use and kept, so drawing a frame never touches pixel data).
 */
export class SnakeSpriteSheet {
  private readonly tintedHeads = new Map<string, HTMLCanvasElement>();

  private constructor(
    private readonly apple: HTMLCanvasElement,
    private readonly head: HTMLCanvasElement,
  ) {}

  /** `null` when no 2D canvas is available. */
  static cutFrom(image: CanvasImageSource): SnakeSpriteSheet | null {
    const apple = cutOut(image, APPLE_CELL);
    const head = cutOut(image, HEAD_FACING_RIGHT_CELL);
    return apple && head ? new SnakeSpriteSheet(apple.canvas, head.canvas) : null;
  }

  /** The apple in the square at (`x`, `y`). */
  drawApple(context: CanvasRenderingContext2D, x: number, y: number, size: number): void {
    context.drawImage(this.apple, x, y, size, size);
  }

  /** The head centred on (`x`, `y`), turned to `heading` radians (0 = facing right). */
  drawHead(
    context: CanvasRenderingContext2D,
    color: RgbColor,
    x: number,
    y: number,
    size: number,
    heading: number,
  ): void {
    context.save();
    context.translate(x, y);
    context.rotate(heading);
    context.drawImage(this.tinted(color), -size / 2, -size / 2, size, size);
    context.restore();
  }

  private tinted(color: RgbColor): HTMLCanvasElement {
    const key = cssColor(color);
    const cached = this.tintedHeads.get(key);
    if (cached) return cached;
    const copy = cutOut(this.head, { column: 0, row: 0 }, { willReadFrequently: true });
    if (!copy) return this.head;
    const pixels = copy.context.getImageData(0, 0, SPRITE_PIXELS, SPRITE_PIXELS);
    const data = pixels.data;
    for (let offset = 0; offset < data.length; offset += 4) {
      const tinted = tintedBySprite(color, {
        red: data[offset] ?? 0,
        green: data[offset + 1] ?? 0,
        blue: data[offset + 2] ?? 0,
      });
      data[offset] = tinted.red;
      data[offset + 1] = tinted.green;
      data[offset + 2] = tinted.blue;
    }
    copy.context.putImageData(pixels, 0, 0);
    this.tintedHeads.set(key, copy.canvas);
    return copy.canvas;
  }
}

let loading: Promise<SnakeSpriteSheet> | null = null;

/** Loads `public/sprites.png` once per page. */
export function loadSnakeSpriteSheet(): Promise<SnakeSpriteSheet> {
  loading ??= new Promise((resolve, reject) => {
    const image = new Image();
    const fail = () => {
      loading = null;
      reject(new Error("snake sprites failed to load"));
    };
    image.onload = () => {
      const sheet = SnakeSpriteSheet.cutFrom(image);
      if (sheet) resolve(sheet);
      else fail();
    };
    image.onerror = fail;
    image.src = `${import.meta.env.BASE_URL}sprites.png`;
  });
  return loading;
}
