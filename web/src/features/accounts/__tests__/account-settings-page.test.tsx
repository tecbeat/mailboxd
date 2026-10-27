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
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeAll, describe, expect, it, vi } from 'vitest'
import type { AccountModel } from '@/api/account/api'
import { AccountSettingsPage } from '../account-settings-page'

vi.mock('@/hooks/use-toast', () => ({ toast: vi.fn() }))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => <a>{children}</a>,
}))

const BASE = 'http://localhost:15630'

const imapAccount = (over: Partial<AccountModel> = {}): AccountModel => ({
  id: 11,
  account_type: 'IMAP',
  imap: {
    host: 'imap.example.com',
    port: 993,
    encryption: 'Ssl',
    auth: { auth_type: 'Password' },
  },
  enabled: true,
  email: 'imap@example.com',
  download_folders: [],
  created_by: 1,
  created_user_name: 'admin',
  created_user_email: 'admin@example.com',
  created_at: 0,
  updated_at: 0,
  use_dangerous: false,
  ...over,
})

/** Serve `account` and capture what the page saves for it. */
function serve(account: AccountModel) {
  const sent: { body?: Record<string, unknown> } = {}
  server.use(
    http.get(`${BASE}/api/v1/accounts`, () =>
      HttpResponse.json({ items: [account] })
    ),
    http.post(`${BASE}/api/v1/account/${account.id}`, async ({ request }) => {
      sent.body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json({})
    })
  )
  return sent
}

async function save(sent: { body?: Record<string, unknown> }) {
  await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))
  await waitFor(() => expect(sent.body).toBeDefined())
  return sent.body!
}

describe('AccountSettingsPage retention', () => {
  beforeAll(() => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      }
    )
  })

  it('shows the stored period in its own section', async () => {
    serve(imapAccount({ retention_days: 90 }))
    renderWithProviders(<AccountSettingsPage accountId={11} />)

    expect(await screen.findByLabelText('Retention period (days)')).toHaveValue(
      90
    )
    expect(
      screen.getByText('Choose how long archived mail is kept.')
    ).toBeInTheDocument()
  })

  it('turns retention off with 0 when the field is cleared', async () => {
    const sent = serve(imapAccount({ retention_days: 90 }))
    renderWithProviders(<AccountSettingsPage accountId={11} />)

    await userEvent.clear(
      await screen.findByLabelText('Retention period (days)')
    )
    expect(await save(sent)).toMatchObject({ retention_days: 0 })
  })

  it('sends a newly set period', async () => {
    const sent = serve(imapAccount())
    renderWithProviders(<AccountSettingsPage accountId={11} />)

    const field = await screen.findByLabelText('Retention period (days)')
    expect(field).toHaveValue(null)
    await userEvent.type(field, '365')
    expect(await save(sent)).toMatchObject({ retention_days: 365 })
  })
})
