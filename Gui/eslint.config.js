import eslint from "@eslint/js";
import prettier from "eslint-plugin-prettier/recommended";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
// import reactRefresh from "eslint-plugin-react-refresh";
import storybook from "eslint-plugin-storybook";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["dist"],
  },
  // js & ts
  {
    files: ["**/*.{ts,tsx,js,jsx}"],
    extends: [
      eslint.configs.recommended,
      tseslint.configs.strictTypeChecked,
      tseslint.configs.stylisticTypeChecked,
      react.configs.flat.recommended,
      react.configs.flat["jsx-runtime"],
      reactHooks.configs["recommended-latest"],
      // reactRefresh.configs.vite,
      prettier,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: {
        ...globals.browser,
      },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    settings: { react: { version: "detect" } },
    plugins: {
      react,
    },
    rules: {
      "@typescript-eslint/prefer-nullish-coalescing": "off",
      "@typescript-eslint/no-non-null-assertion": "off",
      "@typescript-eslint/no-unused-vars": "off", // handled by typescript
      "react/prop-types": "off", // we are using typescript
      // "react-refresh/only-export-components": ["warn", { allowConstantExport: true }], // false positives
      "@typescript-eslint/restrict-template-expressions": ["error", { allowNumber: true }],
      "prettier/prettier": "warn",
    },
  },
  // js overrides
  {
    files: ["**/*.{js,jsx}"],
    extends: [tseslint.configs.disableTypeChecked],
  },
  // storybook overrides
  {
    files: ["**/*.stories.{tsx,jsx}"],
    extends: [storybook.configs["flat/recommended"]],
    rules: {
      "@typescript-eslint/no-empty-function": "off",
      "@typescript-eslint/require-await": "off",
    },
  },
);
