import { defineConfig, devices } from '@playwright/test'

const PREVIEW_PORT = 4173

// Set MAILBOXD_E2E_URL to run the instance suite against a running mailboxd
// (e.g. the freshly built container image in CI). Without it, the build suite
// runs against the production `dist/` served by `vite preview`.
const instanceUrl = process.env.MAILBOXD_E2E_URL
// Set MAILBOXD_E2E_BASE_PATH as well when that mailboxd runs below a subpath
// (MAILBOXD_BASE_URL); MAILBOXD_E2E_URL must then end with it plus `/`.
const instanceSuite = process.env.MAILBOXD_E2E_BASE_PATH ? '**/subpath.e2e.ts' : '**/instance.e2e.ts'

export default defineConfig({
  testDir: './e2e',
  testMatch: instanceUrl ? instanceSuite : '**/build.e2e.ts',
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? [['list'], ['junit', { outputFile: 'e2e-results/junit.xml' }]] : 'list',
  outputDir: 'e2e-results/artifacts',
  use: {
    baseURL: instanceUrl ?? `http://127.0.0.1:${PREVIEW_PORT}`,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: instanceUrl
    ? undefined
    : {
        command: `pnpm exec vite preview --host 127.0.0.1 --port ${PREVIEW_PORT} --strictPort`,
        url: `http://127.0.0.1:${PREVIEW_PORT}`,
        reuseExistingServer: false,
        timeout: 60_000,
      },
})
