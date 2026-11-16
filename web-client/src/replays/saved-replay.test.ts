import { describe, expect, it } from "vitest";

import { savedReplaysNewestFirst, type StoredFileInfo } from "./saved-replay";

const file = (name: string, mtime = 0, size = 100): StoredFileInfo => ({ name, type: "file", size, mtime });

describe("savedReplaysNewestFirst", () => {
  it("lists replay files newest first, by the match time in the name", () => {
    const list = savedReplaysNewestFirst([
      file("Snake 2026-11-15 09-00-00 local.mgreplay", Date.UTC(2026, 11, 31)),
      file("2048 2026-11-16 14-05-33 3f9a1c.mgreplay", 0, 2048),
      file("Snake 2026-11-16 08-00-00 local.mgreplay"),
    ]);
    expect(list.map((replay) => replay.fileName)).toEqual([
      "2048 2026-11-16 14-05-33 3f9a1c.mgreplay",
      "Snake 2026-11-16 08-00-00 local.mgreplay",
      "Snake 2026-11-15 09-00-00 local.mgreplay",
    ]);
    expect(list[0]).toEqual({
      fileName: "2048 2026-11-16 14-05-33 3f9a1c.mgreplay",
      gameTitle: "2048",
      playedAt: new Date(2026, 10, 16, 14, 5, 33),
      sizeBytes: 2048,
    });
  });

  it("keeps files with foreign names, dated by their modification time", () => {
    const copiedAt = new Date(2026, 10, 17).getTime();
    const [first, second] = savedReplaysNewestFirst([
      file("Snake 2026-11-16 08-00-00 local.mgreplay"),
      file("from-a-friend.MGREPLAY", copiedAt),
    ]);
    expect(first).toMatchObject({ fileName: "from-a-friend.MGREPLAY", gameTitle: null });
    expect(first?.playedAt.getTime()).toBe(copiedAt);
    expect(second?.gameTitle).toBe("Snake");
  });

  it("ignores folders and other files", () => {
    expect(
      savedReplaysNewestFirst([
        { name: "old.mgreplay", type: "directory", size: 0, mtime: 0 },
        file("notes.txt"),
      ]),
    ).toEqual([]);
  });

  it("orders replays of the same second by name, so the list is stable", () => {
    const list = savedReplaysNewestFirst([
      file("Snake 2026-11-16 08-00-00 bbbbbb.mgreplay"),
      file("Snake 2026-11-16 08-00-00 aaaaaa.mgreplay"),
    ]);
    expect(list.map((replay) => replay.fileName)).toEqual([
      "Snake 2026-11-16 08-00-00 aaaaaa.mgreplay",
      "Snake 2026-11-16 08-00-00 bbbbbb.mgreplay",
    ]);
  });
});
