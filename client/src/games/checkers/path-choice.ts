/** What a tap on the board does to the move being picked. */
export type PathChoice =
  { kind: "ignore" } | { kind: "partial"; selection: number[] } | { kind: "complete"; selection: number[] };

function startsWith(path: readonly number[], prefix: readonly number[]): boolean {
  return prefix.every((square, index) => path[index] === square);
}

/**
 * Picks a move square by square from `paths` (the legal moves; none is a prefix of another,
 * since a capture that can go on must). A tap on the next landing extends the selection; a tap on
 * a later landing jumps there when only one route leads to it; a tap on a piece that can move
 * starts over from it. The selection is complete once it is a whole legal path.
 */
export function choosePath(
  paths: readonly (readonly number[])[],
  selection: readonly number[],
  square: number,
): PathChoice {
  const candidates = paths.filter((path) => startsWith(path, selection));
  const step = selection.length;

  // A landing comes before starting over: a king's capture can end on the square it left.
  if (step > 0) {
    const routes = new Set<string>();
    let reached: number[] | null = null;
    for (const path of candidates) {
      const index = path.indexOf(square, step);
      if (index < 0) continue;
      const route = path.slice(0, index + 1);
      routes.add(route.join(","));
      reached = route;
    }
    const nextLanding = candidates.some((path) => path[step] === square);
    if (nextLanding) reached = [...selection, square];
    if (reached && (nextLanding || routes.size === 1)) {
      const complete = candidates.some((path) => path.length === reached.length && startsWith(path, reached));
      return { kind: complete ? "complete" : "partial", selection: reached };
    }
  }

  if (paths.some((path) => path[0] === square)) {
    if (step === 1 && selection[0] === square) return { kind: "partial", selection: [] };
    return { kind: "partial", selection: [square] };
  }
  return { kind: "ignore" };
}
