// What fit.mjs looks for on a page, as a script run in it: every problem as
// { kind, element, text }. AGENTS.md "Screens": no text clipped, ellipsized or spilling out of
// its control, nothing scrolling sideways.

export const FIT_CHECK = String.raw`(() => {
  const problems = [];
  const describe = (element) => {
    const classes = typeof element.className === "string" ? element.className.trim().split(/\s+/).slice(0, 4).join(".") : "";
    return element.tagName.toLowerCase() + (classes ? "." + classes : "");
  };
  const textOf = (element) => (element.innerText || element.textContent || "").trim().replace(/\s+/g, " ").slice(0, 60);
  const report = (kind, element) => problems.push({ kind, element: describe(element), text: textOf(element) });
  const page = document.documentElement;
  if (page.scrollWidth > window.innerWidth + 1) {
    problems.push({ kind: "page scrolls sideways", element: "html", text: page.scrollWidth + "px wide in " + window.innerWidth + "px" });
  }
  const clips = (value) => value === "hidden" || value === "clip" || value === "auto" || value === "scroll";
  for (const element of document.body.querySelectorAll("*")) {
    if (!(element instanceof HTMLElement)) continue;
    const ownText = [...element.childNodes].some((node) => node.nodeType === Node.TEXT_NODE && node.textContent.trim() !== "");
    if (!ownText) continue;
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    // Not shown, or shown to screen readers only (a 1px clipped box).
    if (style.display === "none" || style.visibility === "hidden" || box.width <= 1 || box.height <= 1) continue;
    if (style.clip !== "auto" || (style.clipPath !== "none" && style.clipPath !== "")) continue;
    if (element.closest("[aria-hidden=true], canvas, svg")) continue;
    if (style.textOverflow === "ellipsis" && element.scrollWidth > element.clientWidth + 1) {
      report("text ellipsized", element);
      continue;
    }
    if (clips(style.overflowX) && element.scrollWidth > element.clientWidth + 1) {
      report("text cut off sideways", element);
      continue;
    }
    // Lists may scroll down; a box that hides what does not fit cuts the text.
    if ((style.overflowY === "hidden" || style.overflowY === "clip") && element.scrollHeight > element.clientHeight + 1) {
      report("text cut off below", element);
      continue;
    }
    const range = document.createRange();
    range.selectNodeContents(element);
    const text = range.getBoundingClientRect();
    if (text.width > 0 && (text.right > box.right + 1 || text.left < box.left - 1)) {
      report("text spills out of its box", element);
      continue;
    }
    const control = element.closest("button, a, label, [role=button], [role=tab], input, select");
    if (control && control !== element) {
      const bounds = control.getBoundingClientRect();
      if (text.right > bounds.right + 1 || text.left < bounds.left - 1 || text.bottom > bounds.bottom + 1) {
        report("text spills out of its control", element);
        continue;
      }
    }
    if (text.width > 0 && (text.right > window.innerWidth + 1 || text.left < -1)) report("text off the screen", element);
  }
  return problems;
})()`;
