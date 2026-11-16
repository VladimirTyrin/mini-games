import { create, type MessageInitShape } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import { Puzzle2048ViewSchema } from "@/proto/minigames/v2/puzzle2048_pb";

import { tileSprites } from "./tile-sprites";

type ViewInit = MessageInitShape<typeof Puzzle2048ViewSchema>;

const view = (tiles: ViewInit["tiles"]) => create(Puzzle2048ViewSchema, { width: 4, height: 4, tiles });

describe("tileSprites", () => {
  it("slides a moved tile from its previous cell", () => {
    const sprites = tileSprites(
      view([
        {
          id: 7,
          value: 2,
          cell: { row: 0, col: 0 },
          origin: { case: "slid", value: { from: { row: 0, col: 3 } } },
        },
      ]),
    );
    expect(sprites).toEqual([
      {
        id: 7,
        value: 2,
        position: { row: 0, col: 0 },
        entrance: { kind: "slide", from: { row: 0, col: 3 } },
      },
    ]);
  });

  it("keeps a tile that stayed put static", () => {
    const sprites = tileSprites(
      view([
        {
          id: 3,
          value: 4,
          cell: { row: 2, col: 1 },
          origin: { case: "slid", value: { from: { row: 2, col: 1 } } },
        },
      ]),
    );
    expect(sprites.map((sprite) => sprite.entrance)).toEqual([{ kind: "static" }]);
  });

  it("slides both merge sources into the merged cell before the merged tile pops", () => {
    const sprites = tileSprites(
      view([
        {
          id: 9,
          value: 8,
          cell: { row: 0, col: 0 },
          origin: {
            case: "merged",
            value: {
              sources: [
                { id: 1, from: { row: 0, col: 1 } },
                { id: 2, from: { row: 0, col: 3 } },
              ],
            },
          },
        },
      ]),
    );
    expect(sprites).toEqual([
      {
        id: 1,
        value: 4,
        position: { row: 0, col: 0 },
        entrance: { kind: "mergeSource", from: { row: 0, col: 1 } },
      },
      {
        id: 2,
        value: 4,
        position: { row: 0, col: 0 },
        entrance: { kind: "mergeSource", from: { row: 0, col: 3 } },
      },
      { id: 9, value: 8, position: { row: 0, col: 0 }, entrance: { kind: "merge" } },
    ]);
  });

  it("marks spawned tiles and skips tiles without a cell", () => {
    const sprites = tileSprites(
      view([
        { id: 4, value: 2, cell: { row: 3, col: 3 }, origin: { case: "spawned", value: {} } },
        { id: 5, value: 2, origin: { case: "spawned", value: {} } },
      ]),
    );
    expect(sprites).toEqual([{ id: 4, value: 2, position: { row: 3, col: 3 }, entrance: { kind: "spawn" } }]);
  });
});
