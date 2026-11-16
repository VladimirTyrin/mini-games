/** The authority refused a command (illegal move); the match state is unchanged. */
export class CommandRejectedError extends Error {
  override readonly name = "CommandRejectedError";
}
