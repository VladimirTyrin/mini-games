import { APPLE_CELL, SPRITE_PIXELS } from "./snake-sprites";

export class SnakeSpriteSheet {
  constructor(private readonly image: HTMLImageElement) {}

  drawApple(context: CanvasRenderingContext2D, x: number, y: number, size: number): void {
    context.drawImage(
      this.image,
      APPLE_CELL.column * SPRITE_PIXELS,
      APPLE_CELL.row * SPRITE_PIXELS,
      SPRITE_PIXELS,
      SPRITE_PIXELS,
      x,
      y,
      size,
      size,
    );
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
