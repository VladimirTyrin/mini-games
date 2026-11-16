import type { MatchHeader } from "@/proto/minigames/v2/match_pb";

export const REPLAY_FILE_EXTENSION = ".mgreplay";

/** What a replay's file name says: "Snake 2026-11-16 14-05-33 3f9a1c.mgreplay". */
export interface ReplayFileNameParts {
  gameTitle: string;
  /** Local wall-clock time the match started, to the second. */
  startedAt: Date;
  /** Start of the match id ("local" for offline matches); with the time it keeps names unique. */
  matchTag: string;
}

// Characters that Android, Windows or a share target may refuse in a file name.
const UNSAFE_CHARACTERS = /[\\/:*?"<>|\p{Cc}]/gu;
const MATCH_TAG_LENGTH = 6;
const FILE_NAME_PATTERN =
  /^(?<title>.+) (?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2}) (?<hours>\d{2})-(?<minutes>\d{2})-(?<seconds>\d{2}) (?<tag>[a-z0-9]+)\.mgreplay$/i;

const twoDigits = (value: number) => String(value).padStart(2, "0");

export function replayMatchTag(matchId: string): string {
  return (
    matchId
      .toLowerCase()
      .replace(/[^a-z0-9]/g, "")
      .slice(0, MATCH_TAG_LENGTH) || "match"
  );
}

export function buildReplayFileName(parts: ReplayFileNameParts): string {
  const title = parts.gameTitle.replace(UNSAFE_CHARACTERS, "").replace(/\s+/g, " ").trim() || "Replay";
  const at = parts.startedAt;
  const date = `${at.getFullYear()}-${twoDigits(at.getMonth() + 1)}-${twoDigits(at.getDate())}`;
  const time = `${twoDigits(at.getHours())}-${twoDigits(at.getMinutes())}-${twoDigits(at.getSeconds())}`;
  return `${title} ${date} ${time} ${replayMatchTag(parts.matchTag)}${REPLAY_FILE_EXTENSION}`;
}

/**
 * Named after the match, not the moment of saving: saving the same match twice overwrites one
 * file instead of piling up copies.
 */
export function replayFileNameForMatch(header: MatchHeader, gameTitle: string): string {
  const startedAtMs = Number(header.startedAtUnixMs);
  return buildReplayFileName({
    gameTitle,
    startedAt: new Date(startedAtMs > 0 ? startedAtMs : Date.now()),
    matchTag: header.matchId,
  });
}

/** `null` for names this app did not build (a renamed or copied-in file). */
export function parseReplayFileName(fileName: string): ReplayFileNameParts | null {
  const groups = FILE_NAME_PATTERN.exec(fileName)?.groups;
  if (!groups?.title || !groups.tag) return null;
  const field = (name: string) => Number(groups[name]);
  const [year, month, day] = [field("year"), field("month"), field("day")];
  const [hours, minutes, seconds] = [field("hours"), field("minutes"), field("seconds")];
  // Date rolls 2026-02-31 over to March: such a name was not built here. The time is checked by
  // range only, so a start inside a daylight-saving gap still parses.
  const calendarDay = new Date(year, month - 1, day);
  const isRealDay =
    calendarDay.getFullYear() === year &&
    calendarDay.getMonth() === month - 1 &&
    calendarDay.getDate() === day;
  if (!isRealDay || hours > 23 || minutes > 59 || seconds > 59) return null;
  return {
    gameTitle: groups.title,
    startedAt: new Date(year, month - 1, day, hours, minutes, seconds),
    matchTag: groups.tag.toLowerCase(),
  };
}
