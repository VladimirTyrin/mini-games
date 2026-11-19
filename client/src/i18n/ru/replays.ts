import type { Messages } from "../messages";

export const replays: Messages["replays"] = {
  title: "Сохранённые повторы",
  noneYet: "Сохранённых повторов пока нет",
  noneYetHint: (saveButton: string) =>
    `Когда матч закончится, нажми «${saveButton}» — повтор появится здесь: его можно пересмотреть или отправить.`,
  imported: "Импортирован",
  share: "Отправить",
  delete: "Удалить",
  keep: "Оставить",
  confirmDelete: "Удалить этот повтор? Это нельзя отменить.",
  deleted: "Повтор удалён",
  cannotOpen: (reason: string) => `Не удалось открыть повтор: ${reason}`,
  whereKept:
    "Повторы хранятся в папке приложения (Android/data/xyz.braintvsminigames.app/files/replays) и удаляются вместе с приложением. Отправь повтор, чтобы сохранить копию в другом месте.",
};
