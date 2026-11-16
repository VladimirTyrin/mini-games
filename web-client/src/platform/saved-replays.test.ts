import { beforeEach, describe, expect, it, vi } from "vitest";

import { deleteSavedReplay, listSavedReplays, readSavedReplay, saveReplayFile } from "./saved-replays";

interface PathOptions {
  path: string;
  directory: string;
}

/** In-memory stand-in for Capacitor's Filesystem: base64 contents keyed by "directory:path". */
const files = new Map<string, string>();
const key = ({ directory, path }: PathOptions) => `${directory}:${path}`;
const missing = () => Promise.reject(new Error("File does not exist"));

vi.mock("@capacitor/filesystem", () => ({
  Directory: { External: "EXTERNAL" },
  Filesystem: {
    writeFile: (options: PathOptions & { data: string }) => {
      files.set(key(options), options.data);
      return Promise.resolve({ uri: `file:///${options.path}` });
    },
    readFile: (options: PathOptions) => {
      const data = files.get(key(options));
      return data === undefined ? missing() : Promise.resolve({ data });
    },
    deleteFile: (options: PathOptions) => (files.delete(key(options)) ? Promise.resolve() : missing()),
    stat: (options: PathOptions) =>
      [...files.keys()].some((name) => name.startsWith(`${key(options)}/`))
        ? Promise.resolve({ type: "directory" })
        : missing(),
    readdir: (options: PathOptions) => {
      const prefix = `${key(options)}/`;
      const entries = [...files.entries()].filter(([name]) => name.startsWith(prefix));
      if (entries.length === 0) return Promise.reject(new Error("Folder does not exist."));
      return Promise.resolve({
        files: entries.map(([name, data]) => ({
          name: name.slice(prefix.length),
          type: "file",
          size: atob(data).length,
          mtime: 0,
        })),
      });
    },
  },
}));

beforeEach(() => files.clear());

describe("saved replays", () => {
  it("lists nothing before the first replay is saved", async () => {
    expect(await listSavedReplays()).toEqual([]);
  });

  it("keeps replays in the app's replays folder and reads them back intact", async () => {
    const bytes = new Uint8Array([0, 1, 2, 250, 255]);
    await saveReplayFile(bytes, "Snake 2026-11-16 14-05-33 local.mgreplay");

    expect([...files.keys()]).toEqual(["EXTERNAL:replays/Snake 2026-11-16 14-05-33 local.mgreplay"]);
    expect(await listSavedReplays()).toMatchObject([{ gameTitle: "Snake", sizeBytes: 5 }]);
    expect(await readSavedReplay("Snake 2026-11-16 14-05-33 local.mgreplay")).toEqual(bytes);
  });

  it("forgets a deleted replay", async () => {
    await saveReplayFile(new Uint8Array([1]), "Snake 2026-11-16 14-05-33 local.mgreplay");
    await saveReplayFile(new Uint8Array([2]), "2048 2026-11-16 15-00-00 local.mgreplay");
    await deleteSavedReplay("Snake 2026-11-16 14-05-33 local.mgreplay");

    expect((await listSavedReplays()).map((replay) => replay.gameTitle)).toEqual(["2048"]);
  });
});
