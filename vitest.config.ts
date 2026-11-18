import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'node:path';

// Some shells export NODE_ENV=production, which makes React resolve its
// production build and removes React.act from @testing-library/react.
process.env.NODE_ENV = 'test';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@shared': path.resolve('./src/shared'),
      '@renderer': path.resolve('./src/renderer'),
      '@features': path.resolve('./src/features'),
      '@lib': path.resolve('./src/lib'),
      '@logging': path.resolve('./src/logging'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    env: { NODE_ENV: 'test' },
    include: ['src/**/__tests__/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
  },
});
