import { pluralEn } from "../plural";

export const online = {
  title: "Online",
  settings: "Settings",
  youAreIn: (lobbyName: string) => `You are in ${lobbyName}`,
  open: "Open",
  friends: "Friends",
  friendRequestCount: (count: number) => `${count} ${pluralEn(count, "request", "requests")}`,
  /** The count read on its own, out of the Friends heading's sight. */
  friendRequestCountLabel: (count: number) =>
    `${count} ${pluralEn(count, "friend request", "friend requests")}`,
  manageFriends: "Manage",
  addFriends: "Add friends",
  noFriendsYet: "No friends yet. Send someone a friend link to play together.",
  lobbies: "Lobbies",
  lobbySeats: (taken: number, max: number) => `${taken}/${max} ${pluralEn(max, "player", "players")}`,
  lobbyWatchers: (count: number) => `${count} watching`,
  phase: {
    open: "open",
    playing: "playing",
    finished: "finished",
    watchingReplay: "watching replay",
  },
  join: "Join",
  watch: "Watch",
  noLobbies: "No lobbies yet — create one!",
  gamePicker: "Game",
  standardSettings: "Standard settings",
  namedSettingsSummary: (lobbyName: string, summary: string) => `“${lobbyName}” · ${summary}`,
  showSettings: "Change settings",
  hideSettings: "Hide settings",
  lobbyNamePlaceholder: "Lobby name (optional)",
  lobbyNameLabel: "Lobby name",
  createLobby: "Create lobby",
};
