import { ref, watch } from "vue";

import { storage } from "@/platform/storage";

const MARK_MISSING_KEY = "mini-games.word-hunt.mark-missing-letters";

/**
 * Darken keys of letters known to be missing from the word. A look preference of this device,
 * not a game setting: it changes no rule, so it stays out of match settings and replays. Off by
 * default: the keyboard then shows only the letters found.
 */
export const marksMissingLetters = ref(storage.get(MARK_MISSING_KEY) === "1");

watch(marksMissingLetters, (marks) => storage.set(MARK_MISSING_KEY, marks ? "1" : "0"));
