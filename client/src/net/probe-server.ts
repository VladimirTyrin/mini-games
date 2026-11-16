import { create, fromJsonString, toJsonString } from "@bufbuild/protobuf";

import type { ServerInfo } from "@/proto/minigames/v2/auth_pb";
import { ClientMessageSchema, ServerMessageSchema } from "@/proto/minigames/v2/protocol_pb";

const PROBE_TIMEOUT_MS = 5_000;

/** Asks a server what sign-in options it offers, without signing in. */
export function probeServer(url: string): Promise<ServerInfo> {
  return new Promise((resolve, reject) => {
    let socket: WebSocket;
    try {
      socket = new WebSocket(url);
    } catch (error) {
      reject(error instanceof Error ? error : new Error(String(error)));
      return;
    }
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error("the server does not answer"));
    }, PROBE_TIMEOUT_MS);
    socket.onopen = () => {
      const request = create(ClientMessageSchema, {
        requestId: 1,
        message: { case: "serverInfo", value: {} },
      });
      socket.send(toJsonString(ClientMessageSchema, request));
    };
    socket.onmessage = (event) => {
      if (typeof event.data !== "string") return;
      const message = fromJsonString(ServerMessageSchema, event.data, { ignoreUnknownFields: true });
      if (message.message.case !== "serverInfo") return;
      clearTimeout(timer);
      socket.close();
      resolve(message.message.value);
    };
    socket.onerror = () => {
      clearTimeout(timer);
      reject(new Error("cannot reach the server"));
    };
  });
}
