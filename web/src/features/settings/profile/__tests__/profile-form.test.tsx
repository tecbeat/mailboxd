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
import { describe, expect, it } from 'vitest'

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'
import type { User } from '@/api/users/api'

import { UserProfileForm } from '../profile-form'

const PASSWORD_PLACEHOLDER = 'Leave blank to keep current password'

function makeUser(overrides: Partial<User> = {}): User {
  return {
    id: 1,
    username: 'alice',
    email: 'alice@example.com',
    global_roles: [],
    global_roles_names: [],
    account_access_map: {},
    account_roles_summary: {},
    global_permissions: [],
    account_permissions: {},
    created_at: 0,
    updated_at: 0,
    totp_enabled: false,
    ...overrides,
  }
}

describe('UserProfileForm password field', () => {
  it('renders the password field for a local user', () => {
    renderWithProviders(<UserProfileForm user={makeUser()} />)

    expect(screen.getByPlaceholderText(PASSWORD_PLACEHOLDER)).toBeInTheDocument()
  })

  it('hides the password field for an SSO user', () => {
    renderWithProviders(<UserProfileForm user={makeUser({ sso_provider: 'oidc' })} />)

    expect(screen.queryByPlaceholderText(PASSWORD_PLACEHOLDER)).not.toBeInTheDocument()
  })

  it('keeps other profile fields editable for an SSO user', () => {
    renderWithProviders(<UserProfileForm user={makeUser({ sso_provider: 'oidc' })} />)

    expect(screen.getByDisplayValue('alice')).toBeInTheDocument()
    expect(screen.getByDisplayValue('alice@example.com')).toBeInTheDocument()
  })
})

describe('UserProfileForm update', () => {
  it('refreshes the notifications after saving, so the password reminder can go away', async () => {
    server.use(
      http.post('http://localhost:15630/api/v1/users/1', () => HttpResponse.json(null)),
    )
    const { queryClient } = renderWithProviders(<UserProfileForm user={makeUser()} />)
    queryClient.setQueryData(['system-notifications'], { default_admin_password: true })

    await userEvent.type(screen.getByPlaceholderText(PASSWORD_PLACEHOLDER), 'a-new-Passw0rd!')
    await userEvent.click(screen.getByRole('button', { name: 'Update profile' }))

    await waitFor(() =>
      expect(queryClient.getQueryState(['system-notifications'])?.isInvalidated).toBe(true),
    )
  })
})
