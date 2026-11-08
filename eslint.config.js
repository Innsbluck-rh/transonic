import stylistic from '@stylistic/eslint-plugin';
import { defineConfig, globalIgnores } from 'eslint/config';
import tseslint from 'typescript-eslint';

export default defineConfig(
  globalIgnores(['public/', 'pkg/', 'dist/', 'build/', 'target/', 'src-tauri/', 'server/', 'third_party/', '.claude/', 'src/bindings.ts']),
  {
    files: ['**/*.{ts,tsx,js,jsx,mjs,cjs}'],
    languageOptions: {
      parser: tseslint.parser,
    },
    plugins: {
      '@typescript-eslint': tseslint.plugin,
      '@stylistic': stylistic,
    },
    rules: {
      '@typescript-eslint/array-type': ['error', { default: 'array' }],
      '@stylistic/jsx-curly-brace-presence': ['error', { props: 'never', children: 'ignore', propElementValues: 'ignore' }],
    },
  }
);
