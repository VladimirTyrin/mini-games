import { describe, expect, it } from "vitest";

import { TwoFingerTap } from "./two-finger-tap";

const finger = (id: number, x = 100, y = 100) => ({ id, x, y });

describe("TwoFingerTap", () => {
  it("takes two fingers down and up together as a tap", () => {
    const tap = new TwoFingerTap();
    tap.down([finger(1)], 0);
    tap.down([finger(1), finger(2, 160)], 30);
    expect(tap.up([finger(2, 160)], 150)).toBe(false);
    expect(tap.up([], 160)).toBe(true);
  });

  it("does not take one finger, a slow press, three fingers or a drag", () => {
    const one = new TwoFingerTap();
    one.down([finger(1)], 0);
    expect(one.up([], 80)).toBe(false);

    const slow = new TwoFingerTap();
    slow.down([finger(1), finger(2)], 0);
    expect(slow.up([], 900)).toBe(false);

    const three = new TwoFingerTap();
    three.down([finger(1), finger(2), finger(3)], 0);
    expect(three.up([], 100)).toBe(false);

    const drag = new TwoFingerTap();
    drag.down([finger(1), finger(2, 160)], 0);
    drag.moved([finger(1, 100, 200), finger(2, 160, 200)]);
    expect(drag.up([], 100)).toBe(false);
  });

  it("starts afresh after each gesture", () => {
    const tap = new TwoFingerTap();
    tap.down([finger(1), finger(2), finger(3)], 0);
    tap.up([], 50);
    tap.down([finger(4), finger(5)], 1000);
    expect(tap.up([], 1100)).toBe(true);
  });
});
