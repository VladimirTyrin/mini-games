import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

import { locale, setLocale } from "@/i18n";
import { FakeSocket } from "@/net/testing/fake-socket";

import SettingsView from "./SettingsView.vue";

function mountSettings() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/", component: SettingsView }],
  });
  return mount(SettingsView, {
    global: {
      plugins: [router],
      // Their own screens and tests; here they would only open sockets and ask the browser.
      stubs: { SignInPanel: true, TestNotifications: true, UpdateBanner: true },
    },
  });
}

describe("SettingsView", () => {
  beforeEach(() => {
    vi.stubGlobal("WebSocket", FakeSocket);
    localStorage.clear();
    setActivePinia(createPinia());
    setLocale("en");
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    setLocale("en");
  });

  it("shows the current language, and the choices only on Change", async () => {
    const view = mountSettings();
    const choices = () => view.findAll("button[lang]");
    expect(choices()).toHaveLength(0);

    await view.get('[data-testid="change-language"]').trigger("click");
    expect(choices().map((choice) => choice.text())).toEqual(["English", "Русский"]);

    await choices()[1]?.trigger("click");
    await flushPromises();
    expect(locale.value).toBe("ru");
    expect(choices()).toHaveLength(0);
    expect(view.text()).toContain("Сменить");
  });

  it("keeps the server address folded, with the server named on the fold", async () => {
    localStorage.setItem("mini-games.prefs.v2", JSON.stringify({ serverUrl: "wss://example.org/ws" }));
    const view = mountSettings();
    const fold = view.get('[data-testid="server-settings"]');
    expect(fold.text()).toContain("Server: example.org");
    expect(view.find("form input").exists()).toBe(false);

    await fold.trigger("click");
    expect(view.find("form input").exists()).toBe(true);
  });
});
