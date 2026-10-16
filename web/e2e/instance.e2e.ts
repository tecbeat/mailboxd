import { expect, test, type Page } from '@playwright/test'

// Runs against a real, freshly started mailboxd (see playwright.config.ts).
// A fresh install provisions the default admin account.
const USERNAME = process.env.MAILBOXD_E2E_USERNAME ?? 'admin'
const PASSWORD = process.env.MAILBOXD_E2E_PASSWORD ?? 'admin@mailboxd'

// Every top-level page of the app with the heading it must render.
const ROUTES: ReadonlyArray<{ path: string; heading: string }> = [
  { path: '/', heading: 'Dashboard' },
  { path: '/accounts', heading: 'Email Accounts' },
  { path: '/accounts/new', heading: 'Add Account' },
  { path: '/search', heading: 'Search' },
  { path: '/import', heading: 'Import' },
  { path: '/attachment', heading: 'Attachments' },
  { path: '/oauth2', heading: 'OAuth2' },
  { path: '/users', heading: 'Users' },
  { path: '/audit-log', heading: 'Audit Log' },
  { path: '/api-docs', heading: 'API Documentation' },
]

const SETTINGS_PAGES = [
  '/settings',
  '/settings/profile',
  '/settings/access',
  '/settings/api-tokens',
  '/settings/appearance',
  '/settings/backup',
  '/settings/configurations',
  '/settings/proxy',
] as const

interface Problems {
  pageErrors: string[]
  consoleErrors: string[]
  serverErrors: string[]
  failedRequests: string[]
}

// Collects everything that indicates a broken page: uncaught exceptions,
// console errors (TanStack Router logs caught render errors there), 5xx API
// responses and requests that never completed (e.g. a missing asset).
function watchForProblems(page: Page): Problems {
  const problems: Problems = {
    pageErrors: [],
    consoleErrors: [],
    serverErrors: [],
    failedRequests: [],
  }
  page.on('pageerror', (error) => problems.pageErrors.push(`${error.name}: ${error.message}`))
  page.on('console', (message) => {
    if (message.type() === 'error') problems.consoleErrors.push(message.text())
  })
  page.on('response', (response) => {
    const missingAsset = response.status() === 404 && new URL(response.url()).pathname.startsWith('/assets/')
    if (response.status() >= 500 || missingAsset) {
      problems.serverErrors.push(`${response.status()} ${response.url()}`)
    }
  })
  page.on('requestfailed', (request) => {
    const reason = request.failure()?.errorText ?? 'failed'
    // Navigating away cancels in-flight requests; that is not a failure.
    if (reason === 'net::ERR_ABORTED') return
    problems.failedRequests.push(`${request.url()}: ${reason}`)
  })
  return problems
}

async function signIn(page: Page): Promise<void> {
  await page.goto('/sign-in')
  await page.getByRole('textbox', { name: 'Username' }).fill(USERNAME)
  await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD)
  await page.getByRole('button', { name: 'Login' }).click()
  await expect(page).not.toHaveURL(/\/sign-in/)
  await expect(page.getByRole('heading', { level: 1, name: 'Dashboard' })).toBeVisible()
}

async function expectNoErrorPage(page: Page): Promise<void> {
  await expect(page.getByText(/^Oops!/)).toHaveCount(0)
}

test('the server serves the app for a deep link and asks to sign in', async ({ page }) => {
  const problems = watchForProblems(page)

  await page.goto('/search')

  await expect(page).toHaveURL(/\/sign-in\?redirect=/)
  await expect(page.getByRole('button', { name: 'Login' })).toBeVisible()
  expect(problems.pageErrors).toEqual([])
  expect(problems.serverErrors).toEqual([])
  expect(problems.failedRequests).toEqual([])
})

test('a wrong password is rejected', async ({ page }) => {
  await page.goto('/sign-in')
  await page.getByRole('textbox', { name: 'Username' }).fill(USERNAME)
  await page.getByRole('textbox', { name: 'Password' }).fill(`${PASSWORD}-wrong`)
  await page.getByRole('button', { name: 'Login' }).click()

  await expect(page).toHaveURL(/\/sign-in/)
  await expect(page.getByRole('button', { name: 'Login' })).toBeVisible()
})

test('the admin signs in and every page renders without errors', async ({ page }) => {
  await signIn(page)
  const problems = watchForProblems(page)

  for (const { path, heading } of ROUTES) {
    await test.step(path, async () => {
      await page.goto(path)
      await expect(page.getByRole('heading', { level: 1, name: heading, exact: true })).toBeVisible()
      await expectNoErrorPage(page)
    })
  }

  for (const path of SETTINGS_PAGES) {
    await test.step(path, async () => {
      await page.goto(path)
      await expect(page).not.toHaveURL(/\/sign-in/)
      await expect(page.getByRole('main')).toBeVisible()
      await expectNoErrorPage(page)
    })
  }

  expect(problems.pageErrors).toEqual([])
  expect(problems.consoleErrors).toEqual([])
  expect(problems.serverErrors).toEqual([])
  expect(problems.failedRequests).toEqual([])
})
