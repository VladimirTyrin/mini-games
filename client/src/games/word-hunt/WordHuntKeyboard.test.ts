import { create } from "@bufbuild/protobuf";
import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import { WordHuntKeyHintSchema, WordHuntLetterMark } from "@/proto/minigames/v2/word_hunt_pb";

import WordHuntKeyboard from "./WordHuntKeyboard.vue";

const hints = [
  create(WordHuntKeyHintSchema, { letter: "a", mark: WordHuntLetterMark.CORRECT }),
  create(WordHuntKeyHintSchema, { letter: "b", mark: WordHuntLetterMark.ABSENT }),
];

function keyboard(marksMissing: boolean) {
  return mount(WordHuntKeyboard, { props: { rows: ["abc"], hints, disabled: false, marksMissing } });
}

function key(wrapper: ReturnType<typeof keyboard>, letter: string) {
  const found = wrapper.findAll("button").find((button) => button.text() === letter);
  if (!found) throw new Error(`no key ${letter}`);
  return found;
}

describe("WordHuntKeyboard", () => {
  it("shows a missing letter like an untried one unless asked to mark it", () => {
    const plain = keyboard(false);
    expect(key(plain, "b").classes()).toEqual(key(plain, "c").classes());
    expect(key(plain, "a").classes()).toContain("bg-emerald-600");

    const marking = keyboard(true);
    expect(key(marking, "b").classes()).not.toEqual(key(marking, "c").classes());
  });

  it("still types a marked missing letter", async () => {
    const wrapper = keyboard(true);
    await key(wrapper, "b").trigger("pointerdown");
    expect(wrapper.emitted("letter")).toEqual([["b"]]);
    expect(key(wrapper, "b").attributes("disabled")).toBeUndefined();
  });
});
