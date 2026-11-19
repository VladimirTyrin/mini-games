import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import SegmentedField from "./SegmentedField.vue";

describe("SegmentedField", () => {
  it("marks the current option and emits the picked one", async () => {
    const wrapper = mount(SegmentedField<number>, {
      props: {
        label: "Walls",
        modelValue: 1,
        options: [
          { value: 1, label: "Death" },
          { value: 2, label: "Wrap" },
        ],
      },
    });
    const [death, wrap] = wrapper.findAll('[role="radio"]');
    expect(death?.attributes("aria-checked")).toBe("true");
    expect(wrap?.attributes("aria-checked")).toBe("false");
    await wrap?.trigger("click");
    expect(wrapper.emitted("update:modelValue")).toEqual([[2]]);
  });

  it("explains the option chosen, not one of them always", async () => {
    const wrapper = mount(SegmentedField<number>, {
      props: {
        label: "Walls",
        modelValue: 1,
        "onUpdate:modelValue": (value: number) => wrapper.setProps({ modelValue: value }),
        options: [
          { value: 1, label: "Death", help: "A snake that hits a wall is out" },
          { value: 2, label: "Wrap", help: "Leave one side, come back on the other" },
        ],
      },
    });
    expect(wrapper.text()).toContain("A snake that hits a wall is out");
    await wrapper.findAll('[role="radio"]')[1]?.trigger("click");
    expect(wrapper.text()).toContain("Leave one side, come back on the other");
    expect(wrapper.text()).not.toContain("A snake that hits a wall is out");
  });
});
