import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";

import { useGameKeys } from "./use-game-keys";

/** A board that records the game keys it gets, next to a button and a chat input. */
function mountBoard(): { keys: string[]; unmount: () => void } {
  const keys: string[] = [];
  const Board = defineComponent({
    setup() {
      useGameKeys((key) => {
        keys.push(key);
        return true;
      });
      return () => h("div", [h("button", { id: "save" }, "Save replay"), h("input", { id: "chat" })]);
    },
  });
  const wrapper = mount(Board, { attachTo: document.body });
  return { keys, unmount: () => wrapper.unmount() };
}

function press(target: Element | null, key: string): void {
  target?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
}

describe("useGameKeys", () => {
  let unmount = () => {};
  afterEach(() => unmount());

  it("leaves Enter and Space on a focused button to the button", () => {
    const board = mountBoard();
    unmount = board.unmount;
    const button = document.querySelector("#save");

    press(button, "Enter");
    press(button, " ");
    press(button, "ArrowLeft");

    expect(board.keys).toEqual(["ArrowLeft"]);
  });

  it("ignores keys typed into an input", () => {
    const board = mountBoard();
    unmount = board.unmount;

    press(document.querySelector("#chat"), "a");
    press(document.body, "a");

    expect(board.keys).toEqual(["a"]);
  });

  it("names a letter key by its place, so WASD work under a Russian layout too", () => {
    const board = mountBoard();
    unmount = board.unmount;
    // W under a Russian layout types «ц»; Escape has no letter place and keeps its name.
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "ц", code: "KeyW", bubbles: true }));
    document.body.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true }),
    );
    expect(board.keys).toEqual(["w", "Escape"]);
  });
});
