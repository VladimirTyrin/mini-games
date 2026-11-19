import { t } from "@/i18n";
import { RequestError } from "@/net/request-error";
import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

/** The words for a server's error code; the server's own text is English for logs only. */
export function errorCodeText(code: ErrorCode): string {
  const texts = t.errors;
  switch (code) {
    case ErrorCode.INTERNAL:
      return texts.internal;
    case ErrorCode.INVALID_REQUEST:
      return texts.invalidRequest;
    case ErrorCode.RATE_LIMITED:
      return texts.rateLimited;
    case ErrorCode.PROTOCOL_UNSUPPORTED:
      return texts.protocolUnsupported;
    case ErrorCode.NOT_SIGNED_IN:
      return texts.notSignedIn;
    case ErrorCode.SIGN_IN_FAILED:
      return texts.signInFailed;
    case ErrorCode.GUESTS_DISABLED:
      return texts.guestsDisabled;
    case ErrorCode.SIGNED_IN_ELSEWHERE:
      return texts.signedInElsewhere;
    case ErrorCode.SERVER_UPDATING:
      return texts.serverUpdating;
    case ErrorCode.LOBBY_NOT_FOUND:
      return texts.lobbyNotFound;
    case ErrorCode.LOBBY_FULL:
      return texts.lobbyFull;
    case ErrorCode.NOT_HOST:
      return texts.notHost;
    case ErrorCode.NOT_IN_LOBBY:
      return texts.notInLobby;
    case ErrorCode.ALREADY_IN_LOBBY:
      return texts.alreadyInLobby;
    case ErrorCode.INVALID_SETTINGS:
      return texts.invalidSettings;
    case ErrorCode.NOT_READY:
      return texts.notReady;
    case ErrorCode.WRONG_PHASE:
      return texts.wrongPhase;
    case ErrorCode.MATCH_NOT_FOUND:
      return texts.matchNotFound;
    case ErrorCode.COMMAND_REJECTED:
      return texts.commandRejected;
    case ErrorCode.INVALID_REPLAY:
      return texts.invalidReplay;
    case ErrorCode.PUSH_UNAVAILABLE:
      return texts.pushUnavailable;
    case ErrorCode.ACCOUNT_REQUIRED:
      return texts.accountRequired;
    case ErrorCode.NOT_FRIENDS:
      return texts.notFriends;
    case ErrorCode.FRIEND_LINK_INVALID:
      return texts.friendLinkInvalid;
    case ErrorCode.INVITE_CLOSED:
      return texts.inviteClosed;
    case ErrorCode.ACCOUNT_NOT_FOUND:
      return texts.accountNotFound;
    default:
      return texts.unspecified;
  }
}

/**
 * Text of anything a `catch` can receive, for the player: a server's refusal in the player's
 * language; any other error's own message (the app's own errors are worded where thrown).
 */
export function errorMessage(error: unknown): string {
  if (error instanceof RequestError) {
    if (error.failure === "timed-out") return t.errors.timedOut;
    if (error.failure === "disconnected") return t.errors.disconnected;
    return errorCodeText(error.code);
  }
  return error instanceof Error ? error.message : String(error);
}
