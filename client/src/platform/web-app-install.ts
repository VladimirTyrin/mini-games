/** iPhone or iPad: iPadOS reports a Mac, touch support tells them apart. */
export function isAppleMobile(): boolean {
  const userAgent = navigator.userAgent;
  return (
    /iphone|ipad|ipod/i.test(userAgent) || (/macintosh/i.test(userAgent) && navigator.maxTouchPoints > 1)
  );
}

/** Started from the home screen rather than a browser tab. */
export function isInstalledWebApp(): boolean {
  return (
    matchMedia("(display-mode: standalone)").matches ||
    (navigator as Navigator & { standalone?: boolean }).standalone === true
  );
}
