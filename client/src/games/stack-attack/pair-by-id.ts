export interface EntityPair<T> {
  readonly before: T;
  readonly after: T;
}

export interface EntityChanges<T> {
  /** Present in both views. */
  readonly kept: EntityPair<T>[];
  readonly appeared: T[];
  readonly vanished: T[];
}

/** Matches entities of two consecutive views by their stable id so their motion can be interpolated. */
export function pairById<T>(
  before: readonly T[],
  after: readonly T[],
  idOf: (entity: T) => number,
): EntityChanges<T> {
  const earlier = new Map(before.map((entity) => [idOf(entity), entity]));
  const kept: EntityPair<T>[] = [];
  const appeared: T[] = [];
  for (const entity of after) {
    const previous = earlier.get(idOf(entity));
    if (previous === undefined) appeared.push(entity);
    else {
      kept.push({ before: previous, after: entity });
      earlier.delete(idOf(entity));
    }
  }
  return { kept, appeared, vanished: [...earlier.values()] };
}
