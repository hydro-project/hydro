import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    pool: 'forks', // Use fork pool to avoid CJS issues
    include: ['**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    exclude: [
      '**/node_modules/**', 
      '**/dist/**', 
      '**/_DEPRECATED_*/**',
      // Explicitly exclude the deprecated directories
      '**/src/components/_DEPRECATED_/**'
    ],
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
      '@/visualizer-v4': path.resolve(__dirname, './src/components/visualizer-v4'),
      '@/shared': path.resolve(__dirname, './src/components/visualizer-v4/shared'),
      '@/core': path.resolve(__dirname, './src/components/visualizer-v4/core'),
      '@/bridges': path.resolve(__dirname, './src/components/visualizer-v4/bridges'),
      '@/components': path.resolve(__dirname, './src/components/visualizer-v4/components')
    }
  }
});
