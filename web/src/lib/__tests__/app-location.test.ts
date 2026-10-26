//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { describe, expect, it } from 'vitest'
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from '@tanstack/react-router'
import { currentAppLocation, redirectToSignIn, safeAppRedirect } from '../app-location'

async function routerAt(basepath: string, url: string) {
  const rootRoute = createRootRoute()
  const routeTree = rootRoute.addChildren([
    createRoute({ getParentRoute: () => rootRoute, path: '/' }),
    createRoute({ getParentRoute: () => rootRoute, path: '/sign-in' }),
    createRoute({ getParentRoute: () => rootRoute, path: '/accounts' }),
  ])
  const router = createRouter({
    routeTree,
    basepath,
    history: createMemoryHistory({ initialEntries: [url] }),
  })
  await router.load()
  return router
}

function searchParam(router: Awaited<ReturnType<typeof routerAt>>, name: string) {
  return new URLSearchParams(router.history.location.search).get(name)
}

describe('currentAppLocation', () => {
  it('strips the base path from the browser location', async () => {
    const router = await routerAt('/mail', '/mail/accounts?page=2')

    expect(currentAppLocation(router)).toEqual({
      pathname: '/accounts',
      href: '/accounts?page=2',
    })
  })

  it('keeps the location as is without a base path', async () => {
    const router = await routerAt('/', '/accounts')

    expect(currentAppLocation(router).pathname).toBe('/accounts')
  })
})

describe('redirectToSignIn', () => {
  it.each(['/', '/mail', '/apps/mail'])(
    'sends the app path, not the base path, as redirect target (base %s)',
    async (base) => {
      const prefix = base === '/' ? '' : base
      const router = await routerAt(base, `${prefix}/accounts?page=2`)

      redirectToSignIn(router)
      await router.latestLoadPromise

      expect(router.history.location.pathname).toBe(`${prefix}/sign-in`)
      expect(searchParam(router, 'redirect')).toBe('/accounts?page=2')
    }
  )

  it('does not redirect again when already on the sign-in page below a subpath', async () => {
    const router = await routerAt('/mail', '/mail/sign-in?redirect=%2Faccounts')

    redirectToSignIn(router)
    await router.latestLoadPromise

    expect(router.history.location.pathname).toBe('/mail/sign-in')
    expect(searchParam(router, 'redirect')).toBe('/accounts')
  })

  it('lands on the app path after login without doubling the base path', async () => {
    const router = await routerAt('/mail', '/mail/sign-in?redirect=%2Faccounts')

    await router.navigate({ href: safeAppRedirect(searchParam(router, 'redirect')) })

    expect(router.history.location.pathname).toBe('/mail/accounts')
  })

  it('still resolves legacy redirect targets that include the base path', async () => {
    const router = await routerAt('/mail', '/mail/sign-in?redirect=%2Fmail%2F')

    await router.navigate({ href: safeAppRedirect(searchParam(router, 'redirect')) })

    expect(router.history.location.pathname).toBe('/mail/')
  })
})

describe('safeAppRedirect', () => {
  it.each([
    ['/accounts?page=2', '/accounts?page=2'],
    [null, '/'],
    ['', '/'],
    ['https://evil.example', '/'],
    ['//evil.example', '/'],
    ['/\\evil.example', '/'],
  ])('%s -> %s', (input, expected) => {
    expect(safeAppRedirect(input)).toBe(expected)
  })
})
