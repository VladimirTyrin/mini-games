// The app's replay folder. Native only: every caller is behind `__NATIVE_APP__`, so the web build
// drops this module together with its Capacitor imports.
import { type SavedReplay, savedReplaysNewestFirst } from "@/replays/saved-replay";

import { fromBase64, toBase64 } from "./base64";

const REPLAYS_FOLDER = "replays";

export type ShareOutcome = "shared" | "cancelled";

/**
 * `Directory.External` is the app's own folder `Android/data/<package>/files`. Like
 * `Directory.Data` it needs no storage permission and goes away with the app, but unlike Data the
 * player can still reach the files from a computer over USB (and from file managers up to
 * Android 10), and Capacitor's FileProvider (`external-path` in `res/xml/file_paths.xml`) already
 * covers it, so a saved replay goes to the share sheet as it is, without a copy.
 */
async function replayFolder() {
  const { Directory, Filesystem } = await import("@capacitor/filesystem");
  return { Filesystem, directory: Directory.External };
}

const pathOf = (fileName: string) => `${REPLAYS_FOLDER}/${fileName}`;

export async function saveReplayFile(bytes: Uint8Array, fileName: string): Promise<void> {
  const { Filesystem, directory } = await replayFolder();
  await Filesystem.writeFile({ path: pathOf(fileName), data: toBase64(bytes), directory, recursive: true });
}

export async function listSavedReplays(): Promise<SavedReplay[]> {
  const { Filesystem, directory } = await replayFolder();
  try {
    const { files } = await Filesystem.readdir({ path: REPLAYS_FOLDER, directory });
    return savedReplaysNewestFirst(files);
  } catch (error) {
    // The folder appears with the first saved replay; any other failure is real.
    const folderExists = await Filesystem.stat({ path: REPLAYS_FOLDER, directory }).then(
      () => true,
      () => false,
    );
    if (!folderExists) return [];
    throw error;
  }
}

export async function readSavedReplay(fileName: string): Promise<Uint8Array> {
  const { Filesystem, directory } = await replayFolder();
  const { data } = await Filesystem.readFile({ path: pathOf(fileName), directory });
  return typeof data === "string" ? fromBase64(data) : new Uint8Array(await data.arrayBuffer());
}

/**
 * Whether the folder already holds these very bytes, under any name: a local match's header has
 * no start time, so an imported copy of one is named after the moment of import and the name
 * alone would let the same file pile up. Only files of the same size are read.
 */
export async function isAlreadySaved(bytes: Uint8Array): Promise<boolean> {
  for (const replay of await listSavedReplays()) {
    if (replay.sizeBytes !== bytes.length) continue;
    const saved = await readSavedReplay(replay.fileName);
    if (saved.every((byte, index) => byte === bytes[index])) return true;
  }
  return false;
}

export async function shareSavedReplay(fileName: string): Promise<ShareOutcome> {
  const [{ Filesystem, directory }, { Share }] = await Promise.all([
    replayFolder(),
    import("@capacitor/share"),
  ]);
  const { uri } = await Filesystem.getUri({ path: pathOf(fileName), directory });
  try {
    await Share.share({ title: fileName, files: [uri] });
    return "shared";
  } catch {
    // The plugin rejects when the sheet is dismissed; there is nothing else it reports.
    return "cancelled";
  }
}

export async function deleteSavedReplay(fileName: string): Promise<void> {
  const { Filesystem, directory } = await replayFolder();
  await Filesystem.deleteFile({ path: pathOf(fileName), directory });
}
