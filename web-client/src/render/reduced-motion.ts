let query: MediaQueryList | null | undefined;

/**
 * The user asked for less motion: renderers skip decorative movement (squash, bob, particles,
 * shake) but keep motion that carries game information. Read live, so toggling the OS setting
 * applies without a reload.
 */
export function prefersReducedMotion(): boolean {
  if (query === undefined)
    query =
      typeof window.matchMedia === "function" ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  return query?.matches ?? false;
}
