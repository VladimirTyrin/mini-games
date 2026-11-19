export const replays = {
  title: "Saved replays",
  noneYet: "No saved replays yet",
  noneYetHint: (saveButton: string) =>
    `When a match ends, tap “${saveButton}” and it shows up here, ready to watch again or share.`,
  imported: "Imported",
  share: "Share",
  delete: "Delete",
  keep: "Keep",
  confirmDelete: "Delete this replay? This cannot be undone.",
  deleted: "Replay deleted",
  cannotOpen: (reason: string) => `Cannot open the replay: ${reason}`,
  whereKept:
    "Kept in the app's folder (Android/data/xyz.braintvsminigames.app/files/replays) and removed if you uninstall the app. Share a replay to keep a copy elsewhere.",
};
