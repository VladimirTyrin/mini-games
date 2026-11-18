import { create } from "@bufbuild/protobuf";
import { describe, expect, it } from "vitest";

import {
  SnakeDeathReason,
  SnakeDirection,
  type SnakeSeatView,
  SnakeViewSchema,
  SnakeWallCollisionMode,
} from "@/proto/minigames/v2/snake_pb";

import { crashesOf } from "./snake-crash";

type Seat = Pick<SnakeSeatView, "slot" | "alive" | "deathReason" | "direction"> & {
  body: { x: number; y: number }[];
};

function view(seats: Seat[], walls = SnakeWallCollisionMode.DEATH) {
  return create(SnakeViewSchema, {
    fieldWidth: 10,
    fieldHeight: 10,
    wallCollisionMode: walls,
    snakes: seats,
  });
}

const alive = (slot: number): Seat => ({
  slot,
  alive: true,
  deathReason: SnakeDeathReason.UNSPECIFIED,
  direction: SnakeDirection.RIGHT,
  body: [{ x: 0, y: slot }],
});

function crashed(slot: number, head: { x: number; y: number }, direction: SnakeDirection): Seat {
  return { slot, alive: false, deathReason: SnakeDeathReason.SNAKE_COLLISION, direction, body: [head] };
}

describe("crashesOf", () => {
  it("puts a crash into a wall just inside the field's edge in front of the head", () => {
    const wall = {
      ...crashed(0, { x: 9, y: 4 }, SnakeDirection.RIGHT),
      deathReason: SnakeDeathReason.WALL_COLLISION,
    };

    expect(crashesOf(view([wall]), view([alive(0)]))).toEqual([
      { at: { x: 9.9, y: 4.5 }, slots: [0], headOn: false, fresh: true },
    ]);
  });

  it("meets two heads aiming at one cell in that cell", () => {
    const crashes = crashesOf(
      view([
        crashed(0, { x: 3, y: 5 }, SnakeDirection.RIGHT),
        crashed(1, { x: 5, y: 5 }, SnakeDirection.LEFT),
      ]),
      null,
    );

    expect(crashes).toEqual([{ at: { x: 4.5, y: 5.5 }, slots: [0, 1], headOn: true, fresh: false }]);
  });

  it("meets heads swapping cells on the edge between them, also across a wrap-around border", () => {
    const swap = crashesOf(
      view([
        crashed(0, { x: 3, y: 2 }, SnakeDirection.RIGHT),
        crashed(1, { x: 4, y: 2 }, SnakeDirection.LEFT),
      ]),
      null,
    );
    const acrossBorder = crashesOf(
      view(
        [crashed(0, { x: 9, y: 2 }, SnakeDirection.RIGHT), crashed(1, { x: 0, y: 2 }, SnakeDirection.LEFT)],
        SnakeWallCollisionMode.WRAP_AROUND,
      ),
      null,
    );

    expect(swap).toEqual([{ at: { x: 4, y: 2.5 }, slots: [0, 1], headOn: true, fresh: false }]);
    expect(acrossBorder).toEqual([{ at: { x: 0, y: 2.5 }, slots: [0, 1], headOn: true, fresh: false }]);
  });

  it("leaves snakes that left the match out", () => {
    const left = {
      ...crashed(0, { x: 2, y: 2 }, SnakeDirection.UP),
      deathReason: SnakeDeathReason.PLAYER_DISCONNECTED,
    };

    expect(crashesOf(view([left]), null)).toEqual([]);
  });
});
