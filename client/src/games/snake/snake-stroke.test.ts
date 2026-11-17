import { describe, expect, it } from "vitest";

import { sampleCentreline } from "./snake-centreline";
import { strokeVertices } from "./snake-renderer";

describe("strokeVertices", () => {
  // Head at (5, 3), down from (5, 0), then left from the corner to the tail at (0, 0).
  const line = sampleCentreline([
    { x: 5, y: 3 },
    { x: 5, y: 0 },
    { x: 0, y: 0 },
  ]);
  const kept = strokeVertices(line);

  it("keeps both ends and the whole rounded corner", () => {
    expect(kept[0]).toEqual(line[0]);
    expect(kept.at(-1)).toEqual(line.at(-1));
    const corner = line.filter((point) => point.x > 4.5 && point.y < 0.5);
    for (const point of corner) expect(kept).toContainEqual(point);
  });

  it("drops the inner points of the straight runs", () => {
    expect(kept.length).toBeLessThan(line.length / 4);
    const straight = kept.filter((point) => point.y === 0 && point.x > 0.1 && point.x < 4.4);
    expect(straight).toHaveLength(0);
  });
});
