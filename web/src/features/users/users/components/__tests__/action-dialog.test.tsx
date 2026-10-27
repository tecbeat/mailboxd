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

import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, it, vi } from 'vitest'

import type { User } from '@/api/users/api'
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

import { UserActionDialog } from '../action-dialog'

const BASE = 'http://localhost:15630'

// jsdom lacks the layout API the dialog's scroll area uses.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
)

const admin: User = {
  id: 1,
  username: 'admin',
  email: 'admin@example.com',
  global_roles: [1],
  global_roles_names: ['Viewer'],
  account_access_map: {},
  account_roles_summary: {},
  global_permissions: [],
  account_permissions: {},
  created_at: 0,
  updated_at: 0,
  totp_enabled: false,
}

describe('UserActionDialog', () => {
  it('refreshes the notifications after an update, so the password reminder can go away', async () => {
    let updated = false
    server.use(
      http.get(`${BASE}/api/v1/list-roles`, () =>
        HttpResponse.json([
          {
            id: 1,
            name: 'Viewer',
            permissions: [],
            is_builtin: true,
            role_type: 'Global',
            created_at: 0,
            updated_at: 0,
          },
        ]),
      ),
      http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json([])),
      http.post(`${BASE}/api/v1/users/1`, () => {
        updated = true
        return HttpResponse.json(null)
      }),
    )
    const onOpenChange = vi.fn()
    const { queryClient } = renderWithProviders(
      <UserActionDialog open onOpenChange={onOpenChange} currentRow={admin} />,
    )
    queryClient.setQueryData(['system-notifications'], { default_admin_password: true })

    await userEvent.click(await screen.findByRole('button', { name: 'Update User' }))

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false))
    expect(updated).toBe(true)
    expect(queryClient.getQueryState(['system-notifications'])?.isInvalidated).toBe(true)
  })
})
