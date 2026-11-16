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
});
