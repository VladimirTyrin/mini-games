import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { defineComponent, h, ref } from "vue";

import { providePauseControl } from "@/match/pause-control";

import PauseButton from "./PauseButton.vue";

/** The button inside a match screen that offers its pause. */
function mountInMatch(canPause: boolean) {
  const paused = ref(false);
  const Screen = defineComponent({
    setup() {
      providePauseControl({ canPause: ref(canPause), paused, toggle: () => (paused.value = !paused.value) });
      return () => h(PauseButton);
    },
  });
  return { wrapper: mount(Screen), paused };
}

describe("PauseButton", () => {
  it("pauses the match, then gives way to the veil's Resume", async () => {
    const { wrapper, paused } = mountInMatch(true);
    await wrapper.get('[data-testid="pause-button"]').trigger("click");
    expect(paused.value).toBe(true);
    expect(wrapper.find('[data-testid="pause-button"]').exists()).toBe(false);
  });

  it("is not there when the match cannot be paused (online, turn-based)", () => {
    expect(mountInMatch(false).wrapper.find('[data-testid="pause-button"]').exists()).toBe(false);
  });
});
