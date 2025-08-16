import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom', // Use jsdom for React testing
    pool: 'forks', // Use fork pool to avoid CJS issues
    include: ['**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    exclude: ['**/node_modules/**', '**/dist/**', '**/_DEPRECATED_*/**'],
    setupFiles: ['./test-setup.ts'], // Add setup file for testing-library
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      exclude: ['**/_DEPRECATED_*/**', '**/dist/**', '**/*.config.*']
    },
    // Exit cleanly after tests complete (no watch mode by default)
    watch: false
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './'),
      '@/shared': path.resolve(__dirname, './shared'),
      '@/core': path.resolve(__dirname, './core'),
      '@/bridges': path.resolve(__dirname, './bridges'),
      '@/components': path.resolve(__dirname, './components')
    }
  }
});
