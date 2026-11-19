/** Whether the app's server answers at `url` (its WebSocket, which the dev server proxies). */
export function serverIsUp(url) {
  return new Promise((resolve) => {
    let socket;
    const done = (up) => {
      clearTimeout(timer);
      try {
        socket?.close();
      } catch {
        // Closing a socket that never opened.
      }
      resolve(up);
    };
    const timer = setTimeout(() => done(false), 3000);
    try {
      socket = new WebSocket(`${url.replace(/^http/, "ws")}/ws`);
      socket.onopen = () => done(true);
      socket.onerror = () => done(false);
    } catch {
      done(false);
    }
  });
}
