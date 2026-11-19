import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import StepperField from "./StepperField.vue";

/** Mounts a stepper whose v-model is wired back into its props, as a parent would. */
function mountStepper(modelValue: number, limits: { min?: number; max?: number } = {}) {
  const wrapper = mount(StepperField, {
    props: {
      label: "Width",
      modelValue,
      ...limits,
      "onUpdate:modelValue": (value: number): void => {
        void wrapper.setProps({ modelValue: value });
      },
    },
  });
  return {
    wrapper,
    shown: () => (wrapper.get("input").element as HTMLInputElement).value,
    field: () => wrapper.get("input"),
    decrease: () => wrapper.get('button[aria-label="Decrease Width"]'),
    increase: () => wrapper.get('button[aria-label="Increase Width"]'),
  };
}

describe("StepperField", () => {
  it("steps by one and shows the value", async () => {
    const stepper = mountStepper(10, { min: 3, max: 20 });
    await stepper.increase().trigger("click");
    await stepper.increase().trigger("click");
    expect(stepper.shown()).toBe("12");
    await stepper.decrease().trigger("click");
    expect(stepper.wrapper.emitted("update:modelValue")).toEqual([[11], [12], [11]]);
  });

  it("disables each button at its limit", async () => {
    const stepper = mountStepper(4, { min: 3, max: 5 });
    await stepper.decrease().trigger("click");
    expect(stepper.decrease().attributes("disabled")).toBeDefined();
    expect(stepper.increase().attributes("disabled")).toBeUndefined();

    await stepper.increase().trigger("click");
    await stepper.increase().trigger("click");
    expect(stepper.shown()).toBe("5");
    expect(stepper.increase().attributes("disabled")).toBeDefined();
    expect(stepper.decrease().attributes("disabled")).toBeUndefined();
  });

  it("steps a value outside the limits straight back inside", async () => {
    const stepper = mountStepper(40, { min: 3, max: 20 });
    expect(stepper.increase().attributes("disabled")).toBeDefined();
    await stepper.decrease().trigger("click");
    expect(stepper.wrapper.emitted("update:modelValue")).toEqual([[20]]);
  });

  it("is unbounded without limits", async () => {
    const stepper = mountStepper(0);
    await stepper.decrease().trigger("click");
    expect(stepper.wrapper.emitted("update:modelValue")).toEqual([[-1]]);
    expect(stepper.increase().attributes("disabled")).toBeUndefined();
  });

  it("shows the core's message under the row", () => {
    const wrapper = mount(StepperField, {
      props: { label: "Width", modelValue: 2, min: 3, max: 20, error: "Must be between 3 and 20, got 2" },
    });
    expect(wrapper.text()).toContain("Must be between 3 and 20, got 2");
  });

  it("takes a number typed in, within the limits, once it is entered", async () => {
    const stepper = mountStepper(10, { min: 3, max: 20 });
    // Typing: input events alone (setValue would also fire change, as entering does).
    (stepper.field().element as HTMLInputElement).value = "15";
    await stepper.field().trigger("input");
    expect(stepper.wrapper.emitted("update:modelValue")).toBeUndefined();
    await stepper.field().trigger("change");
    expect(stepper.shown()).toBe("15");

    await stepper.field().setValue("99");
    expect(stepper.shown()).toBe("20");
  });

  it("keeps the value when what is typed is not a number", async () => {
    const stepper = mountStepper(10, { min: 3, max: 20 });
    await stepper.field().setValue("abc");
    expect(stepper.shown()).toBe("10");
    expect(stepper.wrapper.emitted("update:modelValue")).toBeUndefined();
  });
});
