import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import Avatar from "./Avatar.vue";

const PICTURE = "https://lh3.googleusercontent.com/a/picture";

describe("Avatar", () => {
  it("shows the picture without sending a referrer", () => {
    const avatar = mount(Avatar, { props: { url: PICTURE, name: "Ada" } });
    const image = avatar.get("img");
    expect(image.attributes("src")).toBe(PICTURE);
    expect(image.attributes("referrerpolicy")).toBe("no-referrer");
  });

  it("falls back to the first letter without a picture", () => {
    const avatar = mount(Avatar, { props: { url: "", name: "  ada" } });
    expect(avatar.find("img").exists()).toBe(false);
    expect(avatar.text()).toBe("A");
  });

  it("falls back to the first letter when the picture fails to load", async () => {
    const avatar = mount(Avatar, { props: { url: PICTURE, name: "Вова" } });
    await avatar.get("img").trigger("error");
    expect(avatar.find("img").exists()).toBe(false);
    expect(avatar.text()).toBe("В");
  });

  it("colours namesakes by player id", () => {
    const tone = (playerId: string) =>
      mount(Avatar, { props: { url: "", name: "Alex", playerId } })
        .get("span")
        .classes()
        .join(" ");
    expect(tone("u_a")).not.toBe(tone("u_b"));
    expect(tone("u_a")).toBe(tone("u_a"));
  });

  it("shows a robot for a bot, never a picture or an initial", () => {
    const avatar = mount(Avatar, { props: { url: "https://example.test/a.png", name: "Bot 1", bot: true } });
    expect(avatar.find("img").exists()).toBe(false);
    expect(avatar.find("svg").exists()).toBe(true);
    expect(avatar.text()).toBe("");
  });
});
