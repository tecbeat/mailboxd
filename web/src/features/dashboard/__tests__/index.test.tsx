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

import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, it, vi } from 'vitest'

import { INITIAL_DASHBOARD_STATS } from '@/api/system/api'
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }))

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@/stores/authStore', () => ({ getToken: () => 'token' }))

// jsdom lacks the layout API the charts use.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
)

import MailArchiveDashboard from '../index'

const BASE = 'http://localhost:15630'

type Search = Record<string, unknown>

describe('dashboard top accounts', () => {
  it('lists the top accounts and searches one on click', async () => {
    server.use(
      http.get(`${BASE}/api/v1/dashboard-stats`, () =>
        HttpResponse.json({
          ...INITIAL_DASHBOARD_STATS,
          top_accounts: [
            { key: 'work@example.com', account_id: 7, count: 12 },
            { key: 'gone@example.com', account_id: 8, count: 3 },
          ],
        }),
      ),
      http.get(`${BASE}/api/v1/minimal-account-list`, () =>
        HttpResponse.json([{ id: 7, email: 'work@example.com', name: 'Work' }]),
      ),
    )
    renderWithProviders(<MailArchiveDashboard />)

    // A named account shows its name; one missing from the list its email.
    await userEvent.click(await screen.findByRole('button', { name: 'Work' }))
    expect(screen.getByRole('button', { name: 'gone@example.com' })).toBeInTheDocument()

    const calls = navigate.mock.calls as [{ to: string; search: (prev: Search) => Search }][]
    const [{ to, search }] = calls[calls.length - 1]
    expect(to).toBe('/search')
    expect(search({})).toMatchObject({ page: 1, q: JSON.stringify({ account_ids: [7] }) })
  })
})
