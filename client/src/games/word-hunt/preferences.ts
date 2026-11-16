import { ref, watch } from "vue";

import { storage } from "@/platform/storage";

const MARK_MISSING_KEY = "mini-games.word-hunt.mark-missing-letters";
const HIGH_CONTRAST_KEY = "mini-games.word-hunt.high-contrast";

/**
 * Darken keys of letters known to be missing from the word. A look preference of this device,
 * not a game setting: it changes no rule, so it stays out of match settings and replays. Off by
 * default: the keyboard then shows only the letters found.
 */
export const marksMissingLetters = ref(storage.get(MARK_MISSING_KEY) === "1");

watch(marksMissingLetters, (marks) => storage.set(MARK_MISSING_KEY, marks ? "1" : "0"));

/**
 * Orange in place and blue elsewhere instead of green and amber, which many colour-blind
 * players cannot tell apart. A look preference of this device, like the one above.
 */
export const usesHighContrast = ref(storage.get(HIGH_CONTRAST_KEY) === "1");

watch(usesHighContrast, (high) => storage.set(HIGH_CONTRAST_KEY, high ? "1" : "0"));
