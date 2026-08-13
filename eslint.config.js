const { defineConfig, globalIgnores } = require('eslint/config');
const expoConfig = require('eslint-config-expo/flat');

module.exports = defineConfig([
  globalIgnores([
    '**/.expo/**',
    '**/coverage/**',
    '**/dist/**',
    '**/node_modules/**',
    'apps/mobile/scripts/**',
  ]),
  expoConfig,
  {
    files: ['apps/mobile/**/*.{ts,tsx}', 'packages/**/*.ts'],
    settings: {
      'import/resolver': {
        typescript: {
          project: ['apps/mobile/tsconfig.json', 'packages/*/tsconfig.json'],
        },
      },
    },
    rules: {
      '@typescript-eslint/consistent-type-imports': [
        'error',
        { prefer: 'type-imports', fixStyle: 'inline-type-imports' },
      ],
      'import/no-cycle': 'error',
    },
  },
]);
