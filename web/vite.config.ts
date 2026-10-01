/// <reference types="vitest" />
import path from 'path'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react-swc'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'

// https://vite.dev/config/
export default defineConfig({
  base: '',
  plugins: [react(), TanStackRouterVite()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    globals: true,
    environment: 'jsdom',
    // Match the jsdom origin to the dev API base (see api/axiosInstance.ts) so
    // test requests are same-origin. Otherwise they are cross-origin and msw v3
    // — which enforces real CORS semantics — issues preflight/CORS checks that
    // msw v2 silently skipped, breaking mutation→invalidation integration tests.
    environmentOptions: {
      jsdom: {
        url: 'http://localhost:15630',
      },
    },
    setupFiles: './src/test/setup.ts',
    css: false,
  },
})
