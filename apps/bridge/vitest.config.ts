import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'json'],
      include: ['electron/**/*.js'],
      exclude: ['**/node_modules/**', '**/tests/**'],
    },
    testTimeout: 10000,
  },
});
