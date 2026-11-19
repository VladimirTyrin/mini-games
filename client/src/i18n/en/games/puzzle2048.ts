export const puzzle2048 = {
  title: "2048",
  tagline: "Slide and merge tiles to reach the target",
  toTarget: (target: number) => `to ${target}`,
  controls: {
    keyboard: "Arrow keys or WASD to slide the tiles",
    touch: "Swipe to slide the tiles",
  },
  settings: {
    targetTile: "Target tile",
    targetTileHelp: "Reach it to win",
    customTarget: (target: number) => `Custom (${target})`,
  },
  board: {
    score: "Score",
    moves: "Moves",
    target: "Target",
    reachedTarget: (target: number) => `You reached ${target}!`,
    noMovesLeft: "No moves left",
    abandoned: "Game abandoned",
  },
};
