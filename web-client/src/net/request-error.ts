import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

/** A request the server answered with ErrorResponse, or that never got an answer. */
export class RequestError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string,
  ) {
    super(message);
    this.name = "RequestError";
  }

  static timeout(): RequestError {
    return new RequestError(ErrorCode.UNSPECIFIED, "the server did not answer in time");
  }

  static disconnected(): RequestError {
    return new RequestError(ErrorCode.UNSPECIFIED, "not connected to the server");
  }
}
