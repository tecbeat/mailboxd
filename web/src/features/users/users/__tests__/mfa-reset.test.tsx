//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { User } from '@/api/users/api'
import { setToken } from '@/stores/authStore'
import Users from '../index'

const { toast } = vi.hoisted(() => ({ toast: vi.fn() }))
vi.mock('@/hooks/use-toast', () => ({ toast }))

const BASE = 'http://localhost:15630/api/v1'

function makeUser(id: number, username: string, totp_enabled: boolean): User {
  return {
    id,
    username,
    email: `${username}@example.com`,
    global_roles: [],
    global_roles_names: [],
    account_access_map: {},
    account_roles_summary: {},
    global_permissions: [],
    account_permissions: {},
    created_at: 0,
    updated_at: 0,
    totp_enabled,
  }
}

const admin = makeUser(1, 'admin', true)
const alice = makeUser(2, 'alice', true)
const bob = makeUser(3, 'bob', false)

let currentUserServed = false

function serveUsers(users: User[]) {
  server.use(http.get(`${BASE}/list-users`, () => HttpResponse.json(users)))
}

function rowOf(username: string): HTMLElement {
  return screen
    .getByRole('cell', { name: username })
    .closest('tr') as HTMLElement
}

async function openRowMenu(
  user: ReturnType<typeof userEvent.setup>,
  username: string
) {
  await user.click(
    within(rowOf(username)).getByRole('button', { name: /open menu/i })
  )
  return screen.findByRole('menu')
}

describe('Users – reset 2FA', () => {
  beforeEach(() => {
    toast.mockReset()
    localStorage.clear()
    currentUserServed = false
    setToken({ success: true, access_token: 'token' })
    server.use(
      http.get(`${BASE}/list-roles`, () => HttpResponse.json([])),
      http.get(`${BASE}/current-user`, () => {
        currentUserServed = true
        return HttpResponse.json(admin)
      })
    )
    serveUsers([admin, alice, bob])
  })

  it('shows the 2FA status per user', async () => {
    renderWithProviders(<Users />)

    await screen.findByRole('cell', { name: 'alice' })
    expect(within(rowOf('alice')).getByText('On')).toBeInTheDocument()
    expect(within(rowOf('bob')).queryByText('On')).not.toBeInTheDocument()
  })

  it('offers the reset only for other users with 2FA on', async () => {
    const user = userEvent.setup()
    renderWithProviders(<Users />)
    await screen.findByRole('cell', { name: 'alice' })
    // The own-account check needs the current user to be loaded.
    await waitFor(() => expect(currentUserServed).toBe(true))

    expect(
      within(await openRowMenu(user, 'alice')).getByText('Reset 2FA')
    ).toBeInTheDocument()
    await user.keyboard('{Escape}')

    expect(
      within(await openRowMenu(user, 'bob')).queryByText('Reset 2FA')
    ).not.toBeInTheDocument()
    await user.keyboard('{Escape}')

    expect(
      within(await openRowMenu(user, 'admin')).queryByText('Reset 2FA')
    ).not.toBeInTheDocument()
  })

  it('resets 2FA and refreshes the list', async () => {
    let resetId: string | undefined
    server.use(
      http.post(`${BASE}/mfa/admin-reset/:id`, ({ params }) => {
        resetId = params.id as string
        serveUsers([admin, { ...alice, totp_enabled: false }, bob])
        return new HttpResponse(null, { status: 200 })
      })
    )
    const user = userEvent.setup()
    renderWithProviders(<Users />)
    await screen.findByRole('cell', { name: 'alice' })

    await user.click(
      within(await openRowMenu(user, 'alice')).getByText('Reset 2FA')
    )
    const dialog = await screen.findByRole('alertdialog')
    expect(within(dialog).getByText(/user alice \(ID: 2\)/)).toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: 'Reset 2FA' }))

    await waitFor(() =>
      expect(within(rowOf('alice')).queryByText('On')).not.toBeInTheDocument()
    )
    expect(resetId).toBe('2')
    expect(toast).toHaveBeenCalledWith({
      title: '2FA Reset',
      description: 'Two-factor authentication for "alice" has been turned off.',
    })
    await waitFor(() =>
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    )
  })

  it('shows an error toast when the reset fails', async () => {
    server.use(
      http.post(`${BASE}/mfa/admin-reset/:id`, () =>
        HttpResponse.json(
          { code: 1, message: 'User not found' },
          { status: 404 }
        )
      )
    )
    const user = userEvent.setup()
    renderWithProviders(<Users />)
    await screen.findByRole('cell', { name: 'alice' })

    await user.click(
      within(await openRowMenu(user, 'alice')).getByText('Reset 2FA')
    )
    const dialog = await screen.findByRole('alertdialog')
    await user.click(within(dialog).getByRole('button', { name: 'Reset 2FA' }))

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith({
        variant: 'destructive',
        title: 'Reset Failed',
        description: 'User not found',
      })
    )
    expect(screen.getByRole('alertdialog')).toBeInTheDocument()
  })
})
