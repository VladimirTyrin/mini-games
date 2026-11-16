// Friend links: https://<site>/f/<code>. The Android app opens them itself (App Links); a browser
// lands on the site with ?friend=<code> (nginx redirects), and a code can also be pasted.

const CODE = /^[0-9a-f]{20}$/;
const IN_TEXT = /(?:\/f\/|[?&]friend=)([0-9a-f]{20})(?![0-9a-f])/;

/** The link to share for `code`, on the site of the server the app talks to. */
export function friendLinkUrl(serverUrl: string, code: string): string {
  const url = new URL(serverUrl);
  url.protocol = url.protocol === "ws:" ? "http:" : "https:";
  url.pathname = `/f/${code}`;
  url.search = "";
  url.hash = "";
  return url.toString();
}

/** The code in a pasted link (or a bare code); null when there is none. */
export function friendLinkCode(text: string): string | null {
  const trimmed = text.trim().toLowerCase();
  if (CODE.test(trimmed)) return trimmed;
  return IN_TEXT.exec(trimmed)?.[1] ?? null;
}

/** A friend link the site was opened with; removed from the address so a reload does not reuse it. */
export function takeFriendCodeFromAddress(): string | null {
  const url = new URL(location.href);
  const code = url.searchParams.get("friend");
  if (code === null) return null;
  url.searchParams.delete("friend");
  history.replaceState(history.state, "", url.href);
  return friendLinkCode(code);
}

/** Calls `handler` with the code of every friend link the Android app is opened with. */
export async function onFriendLinkOpened(handler: (code: string) => void): Promise<void> {
  if (!__NATIVE_APP__) return;
  const { App } = await import("@capacitor/app");
  await App.addListener("appUrlOpen", ({ url }) => {
    const code = friendLinkCode(url);
    if (code) handler(code);
  });
}

/** Opens the share sheet, or copies the link where there is none. */
export async function shareLink(url: string, text: string): Promise<"shared" | "copied"> {
  if (__NATIVE_APP__) {
    const { Share } = await import("@capacitor/share");
    await Share.share({ text, url });
    return "shared";
  }
  if (typeof navigator.share === "function") {
    await navigator.share({ text, url });
    return "shared";
  }
  await navigator.clipboard.writeText(url);
  return "copied";
}
