import { readdirSync } from 'node:fs'
import path from 'node:path'
import { expect, test, type Page } from '@playwright/test'

const ASSETS_DIR = path.resolve(import.meta.dirname, '../dist/assets')

function collectPageErrors(page: Page): string[] {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(`${error.name}: ${error.message}`))
  return errors
}

test.beforeEach(async ({ page }) => {
  await page.route('**/api/**', (route) =>
    route.fulfill({ status: 401, json: { message: 'Unauthorized' } }),
  )
})

test('production build boots and renders the sign-in page', async ({ page }) => {
  const errors = collectPageErrors(page)

  await page.goto('/')

  await expect(page).toHaveURL(/\/sign-in/)
  await expect(page.getByRole('textbox', { name: 'Username' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Login' })).toBeVisible()
  expect(errors).toEqual([])
})

test('every built JS chunk evaluates in the browser', async ({ page }) => {
  const chunks = readdirSync(ASSETS_DIR)
    .filter((file) => file.endsWith('.js'))
    .map((file) => `/assets/${file}`)
  expect(chunks.length).toBeGreaterThan(0)

  const errors = collectPageErrors(page)
  await page.goto('/sign-in')
  await expect(page.getByRole('button', { name: 'Login' })).toBeVisible()

  const failures = await page.evaluate(async (urls) => {
    const failed: string[] = []
    for (const url of urls) {
      try {
        await import(/* @vite-ignore */ url)
      } catch (error) {
        failed.push(`${url}: ${String(error)}`)
      }
    }
    return failed
  }, chunks)

  expect(failures).toEqual([])
  expect(errors).toEqual([])
})
