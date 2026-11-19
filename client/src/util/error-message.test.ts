import { afterEach, describe, expect, it } from "vitest";

import { setLocale } from "@/i18n";
import { RequestError } from "@/net/request-error";
import { ErrorCode } from "@/proto/minigames/v2/protocol_pb";

import { errorMessage } from "./error-message";

describe("errorMessage", () => {
  afterEach(() => setLocale("en"));

  it("words a server's error code in the player's language, not the server's English", () => {
    setLocale("ru");
    expect(errorMessage(new RequestError(ErrorCode.LOBBY_FULL, "no free seat"))).toBe("В лобби нет мест");
  });

  it("shows a code this build does not know in the server's English, else as the raw code", () => {
    const newer = 999 as ErrorCode;
    expect(errorMessage(new RequestError(newer, "the moon is in the wrong phase"))).toBe(
      "the moon is in the wrong phase",
    );
    setLocale("ru");
    expect(errorMessage(new RequestError(newer, ""))).toBe("Что-то пошло не так (999)");
  });
});
