import { cssColor, type RgbColor, tintedBySprite } from "./snake-palette";
import { APPLE_CELL, HEAD_FACING_RIGHT_CELL, SPRITE_PIXELS } from "./snake-sprites";

interface SheetCell {
  readonly column: number;
  readonly row: number;
}

/**
 * The v1 sprite sheet: the apple as drawn, and the head recoloured per seat (one copy per colour,
 * built on first use and kept, so drawing a frame never touches pixel data).
 */
export class SnakeSpriteSheet {
  private readonly tintedSheets = new Map<string, HTMLCanvasElement>();

  constructor(private readonly image: HTMLImageElement) {}

  /** The apple in the square at (`x`, `y`). */
  drawApple(context: CanvasRenderingContext2D, x: number, y: number, size: number): void {
    this.drawCell(context, this.image, APPLE_CELL, x, y, size);
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
    this.drawCell(context, this.tinted(color), HEAD_FACING_RIGHT_CELL, -size / 2, -size / 2, size);
    context.restore();
  }

  private drawCell(
    context: CanvasRenderingContext2D,
    source: CanvasImageSource,
    cell: SheetCell,
    x: number,
    y: number,
    size: number,
  ): void {
    context.drawImage(
      source,
      cell.column * SPRITE_PIXELS,
      cell.row * SPRITE_PIXELS,
      SPRITE_PIXELS,
      SPRITE_PIXELS,
      x,
      y,
      size,
      size,
    );
  }

  private tinted(color: RgbColor): CanvasImageSource {
    const key = cssColor(color);
    const cached = this.tintedSheets.get(key);
    if (cached) return cached;
    const sheet = document.createElement("canvas");
    sheet.width = this.image.naturalWidth;
    sheet.height = this.image.naturalHeight;
    const context = sheet.getContext("2d", { willReadFrequently: true });
    if (!context) return this.image;
    context.drawImage(this.image, 0, 0);
    const pixels = context.getImageData(0, 0, sheet.width, sheet.height);
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
    context.putImageData(pixels, 0, 0);
    this.tintedSheets.set(key, sheet);
    return sheet;
  }
}

let loading: Promise<SnakeSpriteSheet> | null = null;

/** Loads `public/sprites.png` once per page. */
export function loadSnakeSpriteSheet(): Promise<SnakeSpriteSheet> {
  loading ??= new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(new SnakeSpriteSheet(image));
    image.onerror = () => {
      loading = null;
      reject(new Error("snake sprites failed to load"));
    };
    image.src = `${import.meta.env.BASE_URL}sprites.png`;
  });
  return loading;
}
