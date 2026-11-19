/**
 * Default server: the build-time value (Pages, Android) or the host that served the page
 * (server-hosted web, local dev through the Vite proxy).
 */
export function defaultServerUrl(): string {
  const configured = import.meta.env.VITE_SERVER_URL as string | undefined;
  if (configured) return configured;
  const scheme = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${scheme}//${window.location.host}/ws`;
}

/** A bare IP address or localhost: no certificate is issued for one, so it serves plain ws. */
const PLAIN_HOST = /^(localhost|\d{1,3}(\.\d{1,3}){3}|\[[0-9a-f:]+\])(:\d+)?(\/|$)/i;

/**
 * Accepts `host`, `host:port`, `http(s)://host` or a full ws(s) URL; returns a ws(s) URL. A bare
 * domain name means wss (a server under a name has a certificate and redirects plain http); a
 * bare IP address or localhost means ws.
 */
export function normalizeServerUrl(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const scheme = PLAIN_HOST.test(trimmed) ? "ws" : "wss";
  const withScheme = /^[a-z]+:\/\//i.test(trimmed) ? trimmed : `${scheme}://${trimmed}`;
  let url: URL;
  try {
    url = new URL(withScheme);
  } catch {
    return null;
  }
  if (url.protocol === "http:") url.protocol = "ws:";
  if (url.protocol === "https:") url.protocol = "wss:";
  if (url.protocol !== "ws:" && url.protocol !== "wss:") return null;
  if (url.pathname === "/" || url.pathname === "") url.pathname = "/ws";
  return url.toString();
}
