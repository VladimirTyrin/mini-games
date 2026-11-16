import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import { defineComponent, h } from "vue";

import { handleBack, useBackHandler } from "./back-navigation";

function screenWithBack(onBack: () => void) {
  return defineComponent({
    setup() {
      useBackHandler(onBack);
      return () => h("div");
    },
  });
}

describe("back navigation", () => {
  it("reports no handler on a screen without a back action", () => {
    expect(handleBack()).toBe(false);
  });

  it("runs the most recently mounted screen's handler", () => {
    const outer = vi.fn();
    const inner = vi.fn();
    const outerScreen = mount(screenWithBack(outer));
    const innerScreen = mount(screenWithBack(inner));

    expect(handleBack()).toBe(true);
    expect(inner).toHaveBeenCalledOnce();
    expect(outer).not.toHaveBeenCalled();

    innerScreen.unmount();
    handleBack();
    expect(outer).toHaveBeenCalledOnce();

    outerScreen.unmount();
    expect(handleBack()).toBe(false);
  });
});
