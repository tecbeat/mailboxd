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
import { AccountNewPage } from '../account-new'

vi.mock('@/hooks/use-toast', () => ({ toast: vi.fn() }))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children }: { children: React.ReactNode }) => <a>{children}</a>,
  useNavigate: () => vi.fn(),
}))

const BASE = 'http://localhost:15630'

/** Fill in the required fields, set `days` if given, and return the created account. */
async function create(days?: string) {
  const sent: { body?: Record<string, unknown> } = {}
  server.use(
    http.post(`${BASE}/api/v1/account`, async ({ request }) => {
      sent.body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json({})
    })
  )
  renderWithProviders(<AccountNewPage />)

  await userEvent.type(
    screen.getByPlaceholderText('e.g john.doe@example.com'),
    'new@example.com'
  )
  await userEvent.type(
    screen.getByPlaceholderText('e.g imap.example.com'),
    'imap.example.com'
  )
  await userEvent.type(
    screen.getByPlaceholderText('Enter your password'),
    'secret'
  )
  if (days)
    await userEvent.type(screen.getByLabelText('Retention period (days)'), days)
  await userEvent.click(screen.getByRole('button', { name: 'Submit' }))

  await waitFor(() => expect(sent.body).toBeDefined())
  return sent.body!
}

describe('AccountNewPage retention', () => {
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

  it('offers retention in its own section, off by default', () => {
    renderWithProviders(<AccountNewPage />)
    expect(
      screen.getByText('Choose how long archived mail is kept.')
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Retention period (days)')).toHaveValue(null)
  })

  it('creates the account with the chosen retention period', async () => {
    expect(await create('30')).toMatchObject({
      email: 'new@example.com',
      retention_days: 30,
    })
  })

  it('creates the account without retention when the field stays empty', async () => {
    const body = await create()
    expect(body).toMatchObject({
      email: 'new@example.com',
      account_type: 'IMAP',
    })
    expect(body).not.toHaveProperty('retention_days')
  })
})
