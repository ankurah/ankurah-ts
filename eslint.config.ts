import ankurahPlugin from './packages/eslint-plugin-ankurah/src/index.ts';
import tsParser from '@typescript-eslint/parser';

export default [
  {
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/__tests__/**',
      '**/*.test.ts',
      'packages/eslint-plugin-ankurah/**',
    ],
  },
  {
    files: ['packages/**/*.ts'],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    plugins: ankurahPlugin.configs.recommended.plugins,
    rules: ankurahPlugin.configs.recommended.rules,
  },
];
