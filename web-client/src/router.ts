import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

// The browser downloads replays to its own folder; only the app keeps a list of them. The
// `__NATIVE_APP__` literal rather than `isNativeApp`: the bundler does not fold the imported
// constant here, and the web build would keep the screen and the Capacitor Filesystem plugin.
const appOnlyRoutes: RouteRecordRaw[] = __NATIVE_APP__
  ? [{ path: "/replays", name: "replays", component: () => import("./views/SavedReplaysView.vue") }]
  : [];

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
    ...appOnlyRoutes,
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ],
});
