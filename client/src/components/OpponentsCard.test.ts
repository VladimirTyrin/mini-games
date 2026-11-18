import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";

import type { GamePlugin } from "@/games/game-plugin";
import type { BotKind } from "@/proto/minigames/v2/game_pb";

import OpponentsCard from "./OpponentsCard.vue";

const bot = { game: { case: "tictactoe", value: 1 } } as unknown as BotKind;
const plugin = { botLabel: () => "Bot" } as unknown as GamePlugin;

describe("OpponentsCard", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("going to watch fills the table with bots enough to play, and back to playing keeps them", async () => {
    const wrapper = mount(OpponentsCard, {
      props: {
        modelValue: [] as BotKind[],
        watching: false,
        plugin,
        available: [bot],
        minPlayers: 2,
        maxPlayers: 4,
        "onUpdate:modelValue": (bots: BotKind[]) => wrapper.setProps({ modelValue: bots }),
        "onUpdate:watching": (watching: boolean) => wrapper.setProps({ watching }),
      },
    });

    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Watch instead")
      ?.trigger("click");
    await flushPromises();

    expect(wrapper.props("watching")).toBe(true);
    expect(wrapper.props("modelValue")).toHaveLength(2);
    expect(wrapper.text()).toContain("2 / 4 players");
    expect(wrapper.text()).toContain("Watching:");
    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Take a seat")
      ?.trigger("click");
    await flushPromises();
    expect(wrapper.props("watching")).toBe(false);
    expect(wrapper.text()).toContain("3 / 4 players");
  });
});
