import pluginVue from "eslint-plugin-vue";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/**", "src/proto/**", "src/wasm/pkg/**", "android/**", "build/**"] },
  ...tseslint.configs.strict,
  ...pluginVue.configs["flat/recommended"],
  {
    files: ["**/*.vue"],
    languageOptions: { parserOptions: { parser: tseslint.parser } },
  },
  {
    rules: {
      "vue/multi-word-component-names": "off",
      // Layout is Prettier's job; these stylistic rules fight it.
      "vue/max-attributes-per-line": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/multiline-html-element-content-newline": "off",
      "vue/html-self-closing": "off",
      "vue/html-indent": "off",
      "vue/html-closing-bracket-newline": "off",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/consistent-type-imports": "error",
      // Every text a player reads comes from the dictionaries (src/i18n); symbols may stay.
      "vue/no-bare-strings-in-template": [
        "error",
        {
          allowlist: [
            "(", ")", ",", ".", "&", "+", "-", "=", "*", "/", "#", "%", "!", "?", ":", "[", "]",
            "{", "}", "<", ">", "·", "•", "‐", "–", "—", "−", "|", "×", "…", "✓", "✕", "→", "←",
          ],
        },
      ],
    },
  },
);
