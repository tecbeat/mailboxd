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

// Full-page screenshots of every page, kept as CI artifacts for a visual check.
let screenshotIndex = 0
async function capture(page: Page, path: string): Promise<void> {
  await page.waitForLoadState('networkidle')
  const name = path === '/' ? 'dashboard' : path.slice(1).replaceAll('/', '-')
  screenshotIndex += 1
  await page.screenshot({
    path: `e2e-results/screenshots/${String(screenshotIndex).padStart(2, '0')}-${name}.png`,
    fullPage: true,
  })
}

async function expectNoErrorPage(page: Page): Promise<void> {
  await expect(page.getByText(/^Oops!/)).toHaveCount(0)
}

async function apiPost(page: Page, path: string, data: unknown): Promise<void> {
  const token = await page.evaluate(() => {
    const stored = Object.values(localStorage).find((value) => value.includes('accessToken'))
    return stored ? (JSON.parse(stored) as { accessToken: string }).accessToken : ''
  })
  const response = await page.request.post(path, { data, headers: { Authorization: `Bearer ${token}` } })
  expect(response.ok(), `${path}: ${response.status()}`).toBe(true)
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
      await capture(page, path)
    })
  }

  for (const path of SETTINGS_PAGES) {
    await test.step(path, async () => {
      await page.goto(path)
      await expect(page).not.toHaveURL(/\/sign-in/)
      await expect(page.getByRole('main')).toBeVisible()
      await expectNoErrorPage(page)
      await capture(page, path)
    })
  }

  expect(problems.pageErrors).toEqual([])
  expect(problems.consoleErrors).toEqual([])
  expect(problems.serverErrors).toEqual([])
  expect(problems.failedRequests).toEqual([])
})

// GitHub #9: opening a dropdown locks the body scroll; that must not reset the
// content scroll position or move the dropdown away from its trigger.
test('opening a dropdown keeps the scroll position', async ({ page }) => {
  await signIn(page)
  await page.goto('/accounts/new')
  await expect(page.getByRole('heading', { level: 1, name: 'Add Account' })).toBeVisible()

  // CSS locator: Radix hides the page from the accessibility tree once open.
  const triggerId = await page.locator('button[role=combobox]', { hasText: 'Password' }).getAttribute('id')
  const trigger = page.locator(`[id="${triggerId}"]`)
  await trigger.scrollIntoViewIfNeeded()
  const content = page.locator('#content')
  const scrollTop = () => content.evaluate((el) => el.scrollTop)
  const before = await scrollTop()
  expect(before).toBeGreaterThan(0)

  await trigger.click()
  const listbox = page.getByRole('listbox')
  await expect(listbox).toBeVisible()

  expect(await scrollTop()).toBe(before)
  await expect(listbox).toBeInViewport()
  const triggerBox = await trigger.boundingBox()
  const listboxBox = await listbox.boundingBox()
  expect(Math.abs(listboxBox!.y - (triggerBox!.y + triggerBox!.height))).toBeLessThan(20)
  await page.getByRole('option', { name: 'OAuth2' }).click()
  await expect(trigger).toHaveText('OAuth2')
})

// GitHub #10: the account select in the OAuth2 "Authorize" dialog must take
// mouse and keyboard input.
test('an account can be picked in the OAuth2 authorize dialog', async ({ page }) => {
  await signIn(page)
  const suffix = Date.now()
  for (const name of ['first', 'second']) {
    await apiPost(page, '/api/v1/account', {
      email: `${name}-${suffix}@example.test`,
      enabled: false,
      account_type: 'NoSync',
      use_dangerous: false,
    })
  }
  await apiPost(page, '/api/v1/oauth2', {
    description: `e2e-${suffix}`,
    client_id: 'e2e-client',
    client_secret: 'e2e-secret',
    auth_url: 'https://idp.example.test/authorize',
    token_url: 'https://idp.example.test/token',
    redirect_uri: 'http://localhost/oauth2/callback',
    scopes: ['email'],
    enabled: true,
  })

  const openSelect = async () => {
    await page.goto('/oauth2')
    await page.getByRole('row', { name: new RegExp(`e2e-${suffix}`) }).getByRole('button', { name: 'Open menu' }).click()
    await page.getByRole('menuitem', { name: 'Authorize' }).click()
    const dialog = page.getByRole('dialog', { name: 'Authorize Email Account' })
    const trigger = dialog.locator('button[role=combobox]')
    await trigger.click()
    await expect(page.getByRole('option').first()).toBeVisible()
    return { dialog, trigger }
  }

  const byMouse = await openSelect()
  await page.getByRole('option', { name: `second-${suffix}@example.test` }).click()
  await expect(byMouse.trigger).toHaveText(`second-${suffix}@example.test`)
  await expect(byMouse.dialog.getByRole('button', { name: 'Authorize' })).toBeEnabled()

  const byKeyboard = await openSelect()
  await expect(page.getByPlaceholder('Select an account')).toBeFocused()
  await page.keyboard.type(`first-${suffix}`)
  await page.keyboard.press('Enter')
  await expect(byKeyboard.trigger).toHaveText(`first-${suffix}@example.test`)
  await expect(byKeyboard.dialog.getByRole('button', { name: 'Authorize' })).toBeEnabled()
})
