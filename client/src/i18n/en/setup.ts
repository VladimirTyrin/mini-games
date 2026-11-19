import { pluralEn } from "../plural";

export const setup = {
  unknownGame: "Unknown game",
  needsPlayers: (count: number) => `Needs ${count} ${pluralEn(count, "player", "players")}`,
  playWithPeople: "Play with people instead",
};
