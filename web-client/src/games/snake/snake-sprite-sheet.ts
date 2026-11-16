import { cssColor, type RgbColor } from "./snake-palette";
import { SNAKE_SPRITE_CELLS, SPRITE_PIXELS, type SnakeSpriteName } from "./snake-sprites";

// The sheet is drawn in mid greens; brightness scaled by this keeps the outline darker than the
// fill while the fill lands close to the seat colour.
const TINT_GAIN = 1.8;

/**
 * The snake sprite sheet plus one recoloured copy per seat colour, built on first use and kept,
 * so drawing a frame never touches pixel data.
 */
export class SnakeSpriteSheet {
  private readonly tintedSheets = new Map<string, HTMLCanvasElement>();

  constructor(private readonly image: HTMLImageElement) {}

  /** `color` null draws the sheet as is (food). */
  draw(
    context: CanvasRenderingContext2D,
    sprite: SnakeSpriteName,
    color: RgbColor | null,
    x: number,
    y: number,
    size: number,
  ): void {
    const source = color ? this.tinted(color) : this.image;
    const cell = SNAKE_SPRITE_CELLS[sprite];
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
      const brightness =
        (((data[offset] ?? 0) + (data[offset + 1] ?? 0) + (data[offset + 2] ?? 0)) / 765) * TINT_GAIN;
      data[offset] = Math.min(255, color.red * brightness);
      data[offset + 1] = Math.min(255, color.green * brightness);
      data[offset + 2] = Math.min(255, color.blue * brightness);
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
