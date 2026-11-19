import { t } from "@/i18n";
import { CommandRejectedError } from "@/match/command-rejected-error";
import { RequestError } from "@/net/request-error";
import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

import { refusalText, settingProblemText } from "./refusal-text";

/**
 * The words for a server's error code. The server's own English (`english`) is shown only for a
 * code this build does not know (a newer server's).
 */
export function errorCodeText(code: ErrorCode, english?: string): string {
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
      // A code this build does not know: the server's English, else the raw code.
      return english || `${texts.unspecified} (${code})`;
  }
}

/**
 * Text of anything a `catch` can receive, for the player: a server's refusal in the player's
 * language; any other error's own message (the app's own errors are worded where thrown).
 */
export function errorMessage(error: unknown): string {
  // Their messages are the core's English: the fallback for a reason this build cannot word.
  if (error instanceof CommandRejectedError) return refusalText(error.refusal, error.message);
  if (error instanceof RequestError) {
    if (error.failure === "timed-out") return t.errors.timedOut;
    if (error.failure === "disconnected") return t.errors.disconnected;
    if (error.refusal) return refusalText(error.refusal, error.message);
    if (error.settingProblem) return settingProblemText(error.settingProblem, error.message);
    return errorCodeText(error.code, error.message);
  }
  return error instanceof Error ? error.message : String(error);
}
