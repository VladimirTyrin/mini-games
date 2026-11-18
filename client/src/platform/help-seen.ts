import { storage } from "./storage";

const KEY = "mini-games.help-seen";

function seenGames(): string[] {
  try {
    const parsed: unknown = JSON.parse(storage.get(KEY) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

/** Whether this device has already shown `gameId`'s help (opened by itself or by the player). */
export function hasSeenHelp(gameId: string): boolean {
  return seenGames().includes(gameId);
}

export function markHelpSeen(gameId: string): void {
  const seen = seenGames();
  if (!seen.includes(gameId)) storage.set(KEY, JSON.stringify([...seen, gameId]));
}
