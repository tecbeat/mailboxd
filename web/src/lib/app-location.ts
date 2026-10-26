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

import type { AnyRouter } from '@tanstack/react-router'

export const SIGN_IN_PATH = '/sign-in'

/**
 * The browser location as the app sees it: without the base path mailboxd
 * is served under (`/mail/accounts` → `/accounts`). Router `to`/`href`
 * values and `redirect` targets must use this form; the router adds the base
 * path back itself.
 */
export function currentAppLocation(router: AnyRouter): { pathname: string; href: string } {
  const { pathname, href } = router.parseLocation(router.history.location)
  return { pathname, href }
}

export function redirectToSignIn(router: AnyRouter): void {
  const { pathname, href } = currentAppLocation(router)
  if (pathname === SIGN_IN_PATH) return
  router.navigate({ to: SIGN_IN_PATH, search: { redirect: href } })
}

/** Only same-app paths are accepted as a post-login target. */
export function safeAppRedirect(value: string | null | undefined): string {
  if (!value || !value.startsWith('/') || value.startsWith('//') || value.startsWith('/\\')) {
    return '/'
  }
  return value
}
