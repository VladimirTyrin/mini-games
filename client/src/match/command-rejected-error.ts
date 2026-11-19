import type { Refusal } from "@/proto/minigames/v2/refusal_pb";

/**
 * The authority refused a command (illegal move); the match state is unchanged. `refusal` says
 * why (`refusalText` words it); the message is English, for the console.
 */
export class CommandRejectedError extends Error {
  override readonly name = "CommandRejectedError";

  constructor(
    readonly refusal: Refusal | undefined,
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
  }
}
