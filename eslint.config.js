const { defineConfig, globalIgnores } = require('eslint/config');
const expoConfig = require('eslint-config-expo/flat');
const path = require('node:path');

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
          project: [
            path.join(__dirname, 'apps/mobile/tsconfig.json'),
            path.join(__dirname, 'packages/*/tsconfig.json'),
          ],
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
