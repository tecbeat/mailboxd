import { expect, test, type Page } from '@playwright/test'

// Runs against a mailboxd served below a subpath (MAILBOXD_BASE_URL, e.g.
// `/mail`); see playwright.config.ts. Paths are relative to that base, so
// they must not start with `/`.
const BASE_PATH = (process.env.MAILBOXD_E2E_BASE_PATH ?? '/mail').replace(/\/$/, '')
const USERNAME = process.env.MAILBOXD_E2E_USERNAME ?? 'admin'
const PASSWORD = process.env.MAILBOXD_E2E_PASSWORD ?? 'admin@mailboxd'

async function fillLogin(page: Page): Promise<void> {
  await page.getByRole('textbox', { name: 'Username' }).fill(USERNAME)
  await page.getByRole('textbox', { name: 'Password' }).fill(PASSWORD)
  await page.getByRole('button', { name: 'Login' }).click()
}

// GitHub #8: the sign-in redirect must carry the app path, and logging in
// must land on it without doubling the base path.
test('a deep link signs in and returns to the page below the base path', async ({ page }) => {
  await page.goto('search')

  await expect(page).toHaveURL(new RegExp(`${BASE_PATH}/sign-in\\?redirect=%2Fsearch(%3F|$)`))
  await fillLogin(page)

  await expect(page).toHaveURL(new RegExp(`${BASE_PATH}/search(\\?|$)`))
  await expect(page.getByRole('heading', { level: 1, name: 'Search', exact: true })).toBeVisible()
})

// GitHub #11: the OAuth2 callback redirects to the result page below the base
// path, and its links stay there.
test('the OAuth2 callback redirects below the base path', async ({ page }) => {
  await page.goto('sign-in')
  await fillLogin(page)
  await expect(page.getByRole('heading', { level: 1, name: 'Dashboard' })).toBeVisible()

  await page.goto('oauth2/callback')

  await expect(page).toHaveURL(new RegExp(`${BASE_PATH}/oauth2-result\\?error=missing_state`))
  await expect(page.getByRole('link', { name: 'Try Again' })).toHaveAttribute('href', `${BASE_PATH}/oauth2`)
})
