/**
 * Replay files opened with the Android app: "Open with" or Share (MainActivity copies the file
 * into the cache and hands on minigames-replay://open?file=NAME).
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
  if (!__NATIVE_APP__) return;
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
}

function base64Bytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}
