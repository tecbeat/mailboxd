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
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

const { navigate, toast } = vi.hoisted(() => ({ navigate: vi.fn(), toast: vi.fn() }))

vi.mock('@tanstack/react-router', () => ({
  useLocation: () => ({ search: {} }),
  useNavigate: () => navigate,
}))
vi.mock('@/hooks/use-toast', () => ({ toast }))
vi.mock('@/context/theme-context', () => ({ useTheme: () => ({ setTheme: vi.fn() }) }))
vi.mock('@/hooks/use-oidc-config', () => ({
  useOidcConfig: () => ({ oidcEnabled: false, oidcAutoRedirect: false }),
}))

import { UserAuthForm } from '../user-auth-form'

async function logIn() {
  renderWithProviders(<UserAuthForm />)
  await userEvent.type(screen.getByLabelText('Username'), 'admin')
  await userEvent.type(screen.getByPlaceholderText('********'), 'wrong-password')
  await userEvent.click(screen.getByRole('button', { name: 'Login' }))
}

describe('UserAuthForm', () => {
  beforeEach(() => {
    toast.mockReset()
    navigate.mockReset()
  })

  it('tells the user to wait when login is rate-limited', async () => {
    server.use(
      http.post('http://localhost:15630/api/login', () =>
        HttpResponse.json({ message: 'rate limited' }, { status: 429 }),
      ),
    )
    await logIn()

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: 'destructive',
          title: 'Login Failed',
          description: 'Too many login attempts. Please wait a few minutes and try again.',
        }),
      ),
    )
    expect(navigate).not.toHaveBeenCalled()
    // The form is usable again for the next attempt.
    expect(screen.getByRole('button', { name: 'Login' })).toBeEnabled()
  })
})
