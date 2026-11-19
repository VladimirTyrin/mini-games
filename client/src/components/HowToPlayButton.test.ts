import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import HowToPlayButton from "./HowToPlayButton.vue";

/** Whether the button still has the focus after a click with `detail` (0: from the keyboard). */
function keepsFocusAfterClick(detail: number): boolean {
  const wrapper = mount(HowToPlayButton, { attachTo: document.body });
  const button = wrapper.element as HTMLButtonElement;
  button.focus();
  button.dispatchEvent(new MouseEvent("click", { detail, bubbles: true }));
  const focused = document.activeElement === button;
  expect(wrapper.emitted("open")).toHaveLength(1);
  wrapper.unmount();
  return focused;
}

describe("HowToPlayButton", () => {
  it("lets the focus go when clicked with the mouse, so Space pauses the match instead", () => {
    expect(keepsFocusAfterClick(1)).toBe(false);
  });

  it("keeps the focus when pressed from the keyboard", () => {
    expect(keepsFocusAfterClick(0)).toBe(true);
  });
});
