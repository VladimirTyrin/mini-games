/** What a tap on the board does to the move being picked. */
export type PathChoice =
  { kind: "ignore" } | { kind: "partial"; selection: number[] } | { kind: "complete"; selection: number[] };

function startsWith(path: readonly number[], prefix: readonly number[]): boolean {
  return prefix.every((square, index) => path[index] === square);
}

function isWholePath(paths: readonly (readonly number[])[], selection: readonly number[]): boolean {
  return paths.some((path) => path.length === selection.length && startsWith(path, selection));
}

function canGoOn(paths: readonly (readonly number[])[], selection: readonly number[]): boolean {
  return paths.some((path) => path.length > selection.length && startsWith(path, selection));
}

/**
 * Whether the move picked so far may end where it is although it could go on: a capture under
 * the house rule that lets it stop after any jump.
 */
export function canStopHere(paths: readonly (readonly number[])[], selection: readonly number[]): boolean {
  return selection.length > 1 && isWholePath(paths, selection) && canGoOn(paths, selection);
}

/**
 * Picks a move square by square from `paths` (the legal moves). A tap on the next landing
 * extends the selection; a tap on a later landing jumps there when only one route leads to it; a
 * tap on a piece that can move starts over from it. The selection is complete once it is a whole
 * legal path that cannot go on; one that could (the house rule) ends with a tap on the moving
 * piece where it stands.
 */
export function choosePath(
  paths: readonly (readonly number[])[],
  selection: readonly number[],
  square: number,
): PathChoice {
  const candidates = paths.filter((path) => startsWith(path, selection));
  const step = selection.length;

  // A tap on the picked piece puts it back, even where a capture could end on the square it left
  // (that one is walked landing by landing).
  if (step === 1 && selection[0] === square) return { kind: "partial", selection: [] };
  if (step > 1 && selection.at(-1) === square && isWholePath(paths, selection)) {
    return { kind: "complete", selection: [...selection] };
  }
  // Past that, a landing comes before starting over: a capture can end on the square it left.
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
      const complete = isWholePath(paths, reached) && !canGoOn(paths, reached);
      return { kind: complete ? "complete" : "partial", selection: reached };
    }
  }

  if (paths.some((path) => path[0] === square)) return { kind: "partial", selection: [square] };
  return { kind: "ignore" };
}
