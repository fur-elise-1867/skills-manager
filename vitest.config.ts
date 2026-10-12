import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    include: ['src/company/tests/**/*.test.tsx'],
    setupFiles: ['src/company/tests/setup.ts'],
    clearMocks: true,
  },
});
