import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import ChipsField from "./ChipsField.vue";
import { type Preset, matchingPresetKey } from "./presets";

const sizes: Preset<{ width: number; height: number }>[] = [
  { key: "small", label: "Small", detail: "4×4", patch: { width: 4, height: 4 } },
  { key: "large", label: "Large", detail: "6×6", patch: { width: 6, height: 6 } },
];

function mountChips(
  modelValue: string | null,
  extra: { customLabel?: string; unmatchedLabel?: string } = {},
) {
  const wrapper = mount(ChipsField, {
    props: {
      label: "Board size",
      options: sizes,
      modelValue,
      ...extra,
      "onUpdate:modelValue": (value: string | null): void => {
        void wrapper.setProps({ modelValue: value });
      },
    },
    slots: { custom: '<div data-test="custom-controls">steppers</div>' },
  });
  const chip = (text: string) => {
    const found = wrapper.findAll('[role="radio"]').find((candidate) => candidate.text().startsWith(text));
    if (!found) throw new Error(`no chip ${text}`);
    return found;
  };
  return {
    wrapper,
    chip,
    checked: () =>
      wrapper
        .findAll('[role="radio"][aria-checked="true"]')
        .map((candidate) => candidate.attributes("aria-label") ?? candidate.text()),
    showsCustomControls: () => wrapper.find('[data-test="custom-controls"]').exists(),
  };
}

describe("ChipsField", () => {
  it("selects a preset by its key", async () => {
    const chips = mountChips("small");
    expect(chips.checked()).toEqual(["Small, 4×4"]);
    await chips.chip("Large").trigger("click");
    expect(chips.wrapper.emitted("update:modelValue")).toEqual([["large"]]);
    expect(chips.checked()).toEqual(["Large, 6×6"]);
  });

  it("reveals the custom controls only when Custom is picked", async () => {
    const chips = mountChips("small", { customLabel: "Custom" });
    expect(chips.showsCustomControls()).toBe(false);

    await chips.chip("Custom").trigger("click");
    expect(chips.showsCustomControls()).toBe(true);
    expect(chips.checked()).toEqual(["Custom"]);
    expect(chips.wrapper.emitted("update:modelValue")).toBeUndefined();

    await chips.chip("Large").trigger("click");
    expect(chips.showsCustomControls()).toBe(false);
    expect(chips.checked()).toEqual(["Large, 6×6"]);
  });

  it("opens the custom controls when the values match no preset", () => {
    const chips = mountChips(null, { customLabel: "Custom" });
    expect(chips.showsCustomControls()).toBe(true);
    expect(chips.checked()).toEqual(["Custom"]);
  });

  it("names an unmatched value without offering to edit it", () => {
    const chips = mountChips(null, { unmatchedLabel: "Custom (250 ms)" });
    expect(chips.wrapper.text()).toContain("Custom (250 ms)");
    expect(chips.showsCustomControls()).toBe(false);
  });
});

describe("matchingPresetKey", () => {
  it("finds the preset whose values all match", () => {
    expect(matchingPresetKey(sizes, { width: 6, height: 6 })).toBe("large");
    expect(matchingPresetKey(sizes, { width: 6, height: 4 })).toBeNull();
  });
});
