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

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => vi.fn() }))
vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({ require_any_permission: () => true }),
}))

import OAuth2 from '../index'

const BASE = 'http://localhost:15630'

describe('OAuth2 page', () => {
  it('opens the token import from the row menu and closes it again', async () => {
    server.use(
      http.get(`${BASE}/api/v1/oauth2-list`, () =>
        HttpResponse.json({
          items: [
            {
              id: 9,
              description: 'Microsoft',
              client_id: 'client-9',
              client_secret: 'secret',
              auth_url: 'https://login.example.com/authorize',
              token_url: 'https://login.example.com/token',
              redirect_uri: 'https://mailboxd.example.com/oauth2/callback',
              enabled: true,
              created_at: 0,
              updated_at: 0,
            },
          ],
        }),
      ),
      http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json([])),
      // The add dialog is always mounted and loads the proxy list.
      http.get(`${BASE}/api/v1/list-proxy`, () => HttpResponse.json([])),
    )
    renderWithProviders(<OAuth2 />)

    await userEvent.click(await screen.findByRole('button', { name: /open menu/i }))
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Import Tokens' }))

    expect(await screen.findByRole('dialog', { name: 'Import External Tokens' })).toBeInTheDocument()

    await userEvent.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())

    // The row is released after the close animation; it can be opened again.
    await new Promise((resolve) => setTimeout(resolve, 600))
    await userEvent.click(screen.getByRole('button', { name: /open menu/i }))
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Import Tokens' }))
    expect(await screen.findByRole('dialog', { name: 'Import External Tokens' })).toBeInTheDocument()
  })
})
