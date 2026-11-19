import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import { LANGUAGES, pushTextOutputs, readPushTexts, root } from "../../build/push-texts.mjs";

describe("push notification texts", () => {
  it("have every language", () => {
    for (const [key, byLanguage] of Object.entries(readPushTexts())) {
      for (const language of LANGUAGES) expect(byLanguage[language], `${key} in ${language}`).toBeTruthy();
    }
  });

  it("escape what Android resources cannot hold as is (an apostrophe fails the build)", () => {
    const outputs = pushTextOutputs({ push_test: { en: `Sam's "game" <b> & \\`, ru: "x" } });
    expect(outputs["android/app/src/main/res/values/push_strings.xml"]).toContain(
      `<string name="push_test">Sam\\'s \\"game\\" &lt;b&gt; &amp; \\\\</string>`,
    );
  });

  it("are written out as push-texts.json says (npm run generate:push-texts)", () => {
    for (const [path, content] of Object.entries(pushTextOutputs())) {
      expect(readFileSync(resolve(root, path), "utf8"), path).toBe(content);
    }
  });
});
