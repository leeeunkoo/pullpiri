import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react-swc';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      statements: 80,
      branches: 80,
      functions: 80,
      lines: 80,
      include: [
        'src/components/ui/utils.ts',
        'src/components/ui/use-mobile.ts',
        'src/components/ui/use-cluster-health.ts'
      ]
    },
    setupFiles: ['./src/test/setupTests.ts'],
  },
});
