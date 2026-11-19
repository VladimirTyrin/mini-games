import type { Messages } from "../messages";
import { pluralRu } from "../plural";

export const setup: Messages["setup"] = {
  unknownGame: "Неизвестная игра",
  needsPlayers: (count: number) =>
    `${pluralRu(count, "Нужен", "Нужно", "Нужно")} ${count} ${pluralRu(count, "игрок", "игрока", "игроков")}`,
  playWithPeople: "Играть с людьми",
};
