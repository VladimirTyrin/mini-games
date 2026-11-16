import { describe, expect, it } from "vitest";

import { friendLinkCode, friendLinkUrl } from "./friend-links";

const CODE = "0123456789abcdef0123";

describe("friend links", () => {
  it("are made on the site of the server the app talks to", () => {
    expect(friendLinkUrl("wss://braintvsminigames.xyz/ws", CODE)).toBe(
      `https://braintvsminigames.xyz/f/${CODE}`,
    );
    expect(friendLinkUrl("ws://localhost:5173/ws", CODE)).toBe(`http://localhost:5173/f/${CODE}`);
  });

  it("are found in whatever was pasted", () => {
    expect(friendLinkCode(`Add me: https://braintvsminigames.xyz/f/${CODE} see you`)).toBe(CODE);
    expect(friendLinkCode(`https://braintvsminigames.xyz/?friend=${CODE}`)).toBe(CODE);
    expect(friendLinkCode(`  ${CODE.toUpperCase()} `)).toBe(CODE);
  });

  it("are not found in anything else", () => {
    expect(friendLinkCode("https://braintvsminigames.xyz/")).toBeNull();
    expect(friendLinkCode(`https://braintvsminigames.xyz/f/${CODE}ff`)).toBeNull();
    expect(friendLinkCode("hello")).toBeNull();
  });
});
