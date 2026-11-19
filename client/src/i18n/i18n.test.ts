import { afterEach, describe, expect, it } from "vitest";

import { locale, setLocale, t } from "@/i18n";
import { deviceLocale } from "@/i18n/locale";
import { pluralRu } from "@/i18n/plural";

describe("i18n", () => {
  afterEach(() => setLocale("en"));

  it("takes Russian from a Russian device and English from any other", () => {
    expect(deviceLocale(["ru-RU", "en-US"])).toBe("ru");
    expect(deviceLocale(["en-GB", "ru"])).toBe("en");
    expect(deviceLocale(["de"])).toBe("en");
    expect(deviceLocale([])).toBe("en");
  });

  it("speaks the language picked last", () => {
    setLocale("ru");
    expect(t.common.play).toBe("Играть");
    expect(locale.value).toBe("ru");
    setLocale("en");
    expect(t.common.play).toBe("Play");
  });

  it("picks the Russian plural form by the number", () => {
    const wins = (count: number) => pluralRu(count, "победа", "победы", "побед");
    expect([1, 2, 5, 11, 21, 22, 25].map(wins)).toEqual([
      "победа",
      "победы",
      "побед",
      "побед",
      "победа",
      "победы",
      "побед",
    ]);
  });
});
