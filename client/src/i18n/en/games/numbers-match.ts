import { pluralEn } from "../../plural";

export const numbersMatch = {
  title: "Numbers Match",
  tagline: "Pair equal numbers or tens",
  facts: {
    hints: (mode: string) => `hints: ${mode}`,
    refills: (count: number) => `${count} ${pluralEn(count, "refill", "refills")}`,
  },
  controls: {
    keyboard: "Click two equal numbers or two that sum to 10 · H for a hint · R to refill",
    touch: "Tap two equal numbers or two that sum to 10 · Hint and Refill are below the board",
  },
  hintModes: {
    off: "Off",
    limited: "Limited",
    unlimited: "Unlimited",
    unknown: "Default",
  },
  settings: {
    hints: "Hints",
    hintsHelp: "Point out a pair you can take",
    refills: "Refills",
    refillsHelp: "Copy the remaining numbers below",
  },
  board: {
    cleared: "Board cleared!",
    abandoned: "Game abandoned",
    noMovesLeft: "No moves left",
    noPairsRefill: "No pairs left — refill!",
    numbersLeft: (count: number) => `${count} ${pluralEn(count, "number", "numbers")} left`,
    dealing: "Dealing…",
    pairRefused: "That pair cannot be removed",
    refill: "Refill",
    refillWithKey: "Refill (R)",
    hint: "Hint",
    hintWithKey: "Hint (H)",
  },
};
