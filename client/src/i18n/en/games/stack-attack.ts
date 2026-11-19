import { pluralEn } from "../../plural";

export const stackAttack = {
  title: "Stack Attack",
  tagline: "Co-op: survive the falling crates",
  facts: {
    coop: "co-op",
  },
  controls: {
    keyboard:
      "Hold ←/→ or A/D to walk and push crates · Space, W or ↑ to jump · jump, then walk to climb a crate",
    touch:
      "Hold ◀ ▶ to walk and push crates · Jump, then walk to climb a crate · Tap with two fingers to pause",
  },
  endReasons: {
    workersCrushed: "Crushed by a falling crate",
    ceilingReached: "The crates reached the ceiling",
    allWorkersLeft: "Every worker left",
    workersTrapped: "Trapped in a pit with no way out",
  },
  settings: {
    boardSizeHelp: "More room to walk and push crates",
  },
  board: {
    watching: "Watching",
    crushedWatching: "You were crushed — watching",
    walkLeft: "Walk left",
    walkRight: "Walk right",
    jump: "Jump",
  },
  info: {
    score: "Score",
    level: "Level",
    lines: "Lines",
    pushed: "Pushed",
    time: "Time",
    you: "(you)",
    crushed: "crushed",
    left: "left",
    howToScore: (pointsPerLine: number) =>
      `Push crates to fill a row: a full row clears for ${pointsPerLine} ${pluralEn(pointsPerLine, "point", "points")}. Don't get crushed or walled into a pit, and keep the stack below the red line.`,
    smash: "Jump into a falling crate to smash it before it lands.",
  },
};
