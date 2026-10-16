import { defineConfig, devices } from '@playwright/test'

const PORT = 4173

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.e2e.ts',
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? [['list'], ['junit', { outputFile: 'e2e-results/junit.xml' }]] : 'list',
  outputDir: 'e2e-results/artifacts',
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: `pnpm exec vite preview --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
})
