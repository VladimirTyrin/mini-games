import { describe, expect, it } from "vitest";

import { type SwipeDirection, SwipeGesture } from "./swipe-gesture";

/** Swipes a gesture reports as the finger goes through `points`, lifting at the last one. */
function swipesAlong(gesture: SwipeGesture, points: readonly [number, number][]): (SwipeDirection | "tap")[] {
  const reported: (SwipeDirection | "tap")[] = [];
  points.forEach(([x, y], index) => {
    if (index === 0) gesture.down({ x, y });
    else if (index < points.length - 1) {
      const direction = gesture.moved({ x, y });
      if (direction) reported.push(direction);
    } else {
      const outcome = gesture.up({ x, y });
      if (outcome.kind === "swipe") reported.push(outcome.direction);
      else if (outcome.kind === "tap") reported.push("tap");
    }
  });
  return reported;
}

describe("SwipeGesture while dragging", () => {
  it("swipes as soon as the finger has gone far enough, before it lifts", () => {
    const gesture = new SwipeGesture("while-dragging");
    gesture.down({ x: 100, y: 100 });
    expect(gesture.moved({ x: 110, y: 100 })).toBeNull();
    expect(gesture.moved({ x: 130, y: 102 })).toBe("right");
  });

  it("turns again and again in one stroke drawn around a point", () => {
    const stroke: [number, number][] = [
      [100, 100],
      [100, 70],
      [130, 70],
      [130, 100],
      [100, 100],
    ];
    expect(swipesAlong(new SwipeGesture("while-dragging"), stroke)).toEqual(["up", "right", "down", "left"]);
  });

  it("swipes once for a long drag one way", () => {
    const stroke: [number, number][] = [
      [0, 0],
      [30, 0],
      [60, 0],
      [90, 0],
      [120, 0],
    ];
    expect(swipesAlong(new SwipeGesture("while-dragging"), stroke)).toEqual(["right"]);
  });

  it("swipes on lifting when a quick flick reported no moves", () => {
    expect(
      swipesAlong(new SwipeGesture("while-dragging"), [
        [0, 0],
        [0, 40],
      ]),
    ).toEqual(["down"]);
  });

  it("taps when the finger hardly moved", () => {
    expect(
      swipesAlong(new SwipeGesture("while-dragging"), [
        [0, 0],
        [5, 5],
        [8, 3],
      ]),
    ).toEqual(["tap"]);
  });
});

describe("SwipeGesture on release", () => {
  it("waits for the finger to lift and swipes once for the whole gesture", () => {
    const gesture = new SwipeGesture("on-release");
    gesture.down({ x: 0, y: 0 });
    expect(gesture.moved({ x: 0, y: -50 })).toBeNull();
    expect(gesture.moved({ x: 60, y: -50 })).toBeNull();
    expect(gesture.up({ x: 60, y: -40 })).toEqual({ kind: "swipe", direction: "right" });
  });

  it("taps when the finger hardly moved", () => {
    expect(
      swipesAlong(new SwipeGesture("on-release"), [
        [0, 0],
        [10, 10],
      ]),
    ).toEqual(["tap"]);
  });
});

it("forgets a cancelled gesture", () => {
  const gesture = new SwipeGesture("while-dragging");
  gesture.down({ x: 0, y: 0 });
  gesture.cancel();
  expect(gesture.moved({ x: 50, y: 0 })).toBeNull();
  expect(gesture.up({ x: 50, y: 0 })).toEqual({ kind: "nothing" });
});
