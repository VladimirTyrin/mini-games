import { afterEach, describe, expect, it, vi } from "vitest";

import { SnakeSpriteSheet } from "./snake-sprite-sheet";
import { HEAD_FACING_RIGHT_CELL, SPRITE_PIXELS } from "./snake-sprites";

interface DrawImageCall {
  readonly source: CanvasImageSource;
  readonly target: CanvasRenderingContext2D;
  readonly args: readonly number[];
}

const drawn: DrawImageCall[] = [];

/** A 2D context that records its drawImage calls (happy-dom has no canvas). */
function recordingContext(): CanvasRenderingContext2D {
  const context = {
    drawImage: (source: CanvasImageSource, ...args: number[]) =>
      drawn.push({ source, target: context, args }),
    getImageData: (_x: number, _y: number, width: number, height: number) => ({
      data: new Uint8ClampedArray(width * height * 4),
    }),
    putImageData: () => undefined,
    save: () => undefined,
    restore: () => undefined,
    translate: () => undefined,
    rotate: () => undefined,
  } as unknown as CanvasRenderingContext2D;
  return context;
}

afterEach(() => {
  vi.restoreAllMocks();
  drawn.length = 0;
});

describe("SnakeSpriteSheet", () => {
  // Drawn from the sheet with a source rectangle, a head scaled past 64 px blended in the edge of
  // the sprite below it: heading up, a light column along the right of the head's cell.
  it("draws the head from a canvas holding only the head sprite", () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
      () => recordingContext() as unknown as RenderingContext,
    );
    const image = document.createElement("canvas");
    image.width = 5 * SPRITE_PIXELS;
    image.height = 4 * SPRITE_PIXELS;
    const sheet = SnakeSpriteSheet.cutFrom(image);
    expect(sheet).not.toBeNull();

    const board = recordingContext();
    sheet?.drawHead(board, { red: 34, green: 197, blue: 94 }, 467.5, 297.5, 85, -Math.PI / 2);

    const onBoard = drawn.filter((call) => call.target === board);
    expect(onBoard).toHaveLength(1);
    const [headDrawn] = onBoard;
    if (!headDrawn) return;
    // The whole source, scaled to the cell: nothing outside the sprite to sample.
    expect(headDrawn.args).toEqual([-42.5, -42.5, 85, 85]);
    const source = headDrawn.source as HTMLCanvasElement;
    expect([source.width, source.height]).toEqual([SPRITE_PIXELS, SPRITE_PIXELS]);

    // That canvas holds the tinted copy of the head cut out of the sheet.
    const cutsFromSheet = drawn.filter((call) => call.source === image).map((call) => call.args);
    expect(cutsFromSheet).toContainEqual([
      HEAD_FACING_RIGHT_CELL.column * SPRITE_PIXELS,
      HEAD_FACING_RIGHT_CELL.row * SPRITE_PIXELS,
      SPRITE_PIXELS,
      SPRITE_PIXELS,
      0,
      0,
      SPRITE_PIXELS,
      SPRITE_PIXELS,
    ]);
  });
});
