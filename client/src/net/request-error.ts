import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";
import type { Refusal, SettingProblem } from "@/proto/minigames/v2/refusal_pb";

/** Why a request failed: only `refused` carries the server's verdict on the request itself. */
export type RequestFailure = "refused" | "timed-out" | "disconnected";

/**
 * A request the server answered with ErrorResponse, or that never got an answer. The message is
 * the server's English, for the console; `errorMessage` words the code (and the refusal or the
 * setting problem that came with it) for the player.
 */
export class RequestError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string,
    readonly failure: RequestFailure = "refused",
    readonly refusal?: Refusal,
    readonly settingProblem?: SettingProblem,
  ) {
    super(message);
    this.name = "RequestError";
  }

  static timeout(): RequestError {
    return new RequestError(ErrorCode.UNSPECIFIED, "the server did not answer in time", "timed-out");
  }

  static disconnected(): RequestError {
    return new RequestError(ErrorCode.UNSPECIFIED, "not connected to the server", "disconnected");
  }
}
