import type { Messages } from "../messages";
import { pluralRu } from "../plural";

export const online: Messages["online"] = {
  title: "Онлайн",
  settings: "Настройки",
  youAreIn: (lobbyName: string) => `Ты в лобби «${lobbyName}»`,
  open: "Открыть",
  friends: "Друзья",
  newFriendRequests: (count: number) => `${count} ${pluralRu(count, "новая", "новые", "новых")}`,
  lobbies: "Лобби",
  lobbySeats: (taken: number, max: number) => `Игроков: ${taken}/${max}`,
  lobbyWatchers: (count: number) => `зрителей: ${count}`,
  phase: {
    open: "открыто",
    playing: "идёт игра",
    finished: "игра окончена",
    watchingReplay: "смотрят повтор",
  },
  join: "Войти",
  watch: "Смотреть",
  noLobbies: "Лобби пока нет — создай своё",
  gamePicker: "Игра",
  standardSettings: "Обычные настройки",
  namedSettingsSummary: (lobbyName: string, summary: string) => `«${lobbyName}» · ${summary}`,
  showSettings: "Изменить настройки",
  hideSettings: "Скрыть настройки",
  lobbyNamePlaceholder: "Название лобби (необязательно)",
  lobbyNameLabel: "Название лобби",
  createLobby: "Создать лобби",
};
