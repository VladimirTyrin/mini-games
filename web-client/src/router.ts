import { createRouter, createWebHashHistory } from "vue-router";

// Hash history: works unchanged on GitHub Pages, under any base path and inside Capacitor,
// with no server-side SPA fallback.
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: () => import("./views/HomeView.vue") },
    {
      path: "/play/:gameId",
      name: "play",
      component: () => import("./views/LocalSetupView.vue"),
      props: true,
    },
    { path: "/online", name: "online", component: () => import("./views/OnlineView.vue") },
    { path: "/lobby", name: "lobby", component: () => import("./views/LobbyView.vue") },
    { path: "/match", name: "match", component: () => import("./views/MatchView.vue") },
    { path: "/settings", name: "settings", component: () => import("./views/SettingsView.vue") },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});
