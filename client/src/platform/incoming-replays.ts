/**
 * Replay files opened with the app: "Open with" or Share on Android (MainActivity copies the file
 * into the cache and hands on minigames-replay://open?file=NAME), the file handler of an installed
 * web app on a computer (launchQueue).
 */

const SCHEME = "minigames-replay:";
/** As IncomingReplay.DIRECTORY in the Android app, under the cache directory. */
const DIRECTORY = "incoming-replay";

/** The name of the copied file a link names, or `null` when it is not such a link. */
export function incomingReplayFile(url: string): string | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== SCHEME) return null;
    const name = parsed.searchParams.get("file");
    // A plain name inside the directory, nothing that climbs out of it.
    return name && !name.includes("/") && !name.includes("\\") && name !== ".." ? name : null;
  } catch {
    return null;
  }
}

/** Calls `handler` with the bytes of every replay file the app is asked to open. */
export async function onReplayFileOpened(handler: (bytes: Uint8Array) => void): Promise<void> {
  if (__NATIVE_APP__) {
    const [{ App }, { Directory, Filesystem }] = await Promise.all([
      import("@capacitor/app"),
      import("@capacitor/filesystem"),
    ]);
    const open = async (url: string): Promise<void> => {
      const file = incomingReplayFile(url);
      if (!file) return;
      const { data } = await Filesystem.readFile({
        path: `${DIRECTORY}/${file}`,
        directory: Directory.Cache,
      });
      handler(typeof data === "string" ? base64Bytes(data) : new Uint8Array(await data.arrayBuffer()));
    };
    await App.addListener("appUrlOpen", ({ url }) => void open(url));
    // The file the app was started with.
    const launch = await App.getLaunchUrl();
    if (launch?.url) await open(launch.url);
    return;
  }
  // An installed web app on a computer, started from a file (manifest file_handlers).
  const launchQueue = (window as WindowWithLaunchQueue).launchQueue;
  launchQueue?.setConsumer(async ({ files }) => {
    const handle = files[0];
    if (!handle) return;
    const file = await handle.getFile();
    handler(new Uint8Array(await file.arrayBuffer()));
  });
}

function base64Bytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

interface WindowWithLaunchQueue extends Window {
  launchQueue?: {
    setConsumer(consumer: (params: { files: readonly FileSystemFileHandle[] }) => void | Promise<void>): void;
  };
}
