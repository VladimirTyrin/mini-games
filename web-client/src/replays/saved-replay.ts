import { parseReplayFileName, REPLAY_FILE_EXTENSION } from "./replay-file-name";

/** One entry of the app's replay folder, as the "Saved replays" screen lists it. */
export interface SavedReplay {
  fileName: string;
  /** `null` when the name was not built by this app (renamed or copied in by hand). */
  gameTitle: string | null;
  /** Match start from the file name, else the file's modification time. */
  playedAt: Date;
  sizeBytes: number;
}

/** The part of a directory listing entry (Capacitor's `FileInfo`) the list needs. */
export interface StoredFileInfo {
  name: string;
  type: "file" | "directory";
  size: number;
  /** Milliseconds since the epoch. */
  mtime: number;
}

/** Built from the listing alone (names, sizes, times), so no replay file is read. */
export function savedReplaysNewestFirst(files: readonly StoredFileInfo[]): SavedReplay[] {
  return files
    .filter((file) => file.type === "file" && file.name.toLowerCase().endsWith(REPLAY_FILE_EXTENSION))
    .map((file): SavedReplay => {
      const parts = parseReplayFileName(file.name);
      return {
        fileName: file.name,
        gameTitle: parts?.gameTitle ?? null,
        playedAt: parts?.startedAt ?? new Date(file.mtime),
        sizeBytes: file.size,
      };
    })
    .sort(
      (left, right) =>
        right.playedAt.getTime() - left.playedAt.getTime() || left.fileName.localeCompare(right.fileName),
    );
}
