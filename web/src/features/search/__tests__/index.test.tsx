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

import type { EmailEnvelope } from '@/api'
import { DATE_DISPLAY_STORAGE_KEY } from '@/hooks/use-date-display'
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

type Search = {
  page: number
  pageSize: number
  sortBy: 'DATE' | 'SIZE' | 'RELEVANCE'
  sortOrder: 'asc' | 'desc'
  q?: string
}

const { route } = vi.hoisted(() => ({
  route: { search: {} as Search, navigate: vi.fn() },
}))

vi.mock('@tanstack/react-router', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-router')>()),
  getRouteApi: () => ({
    useSearch: () => route.search,
    useNavigate: () => route.navigate,
  }),
  useNavigate: () => vi.fn(),
}))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))

import EmailSearch from '../index'

const BASE = 'http://localhost:15630'
const DATE = Date.UTC(2024, 0, 15, 9, 30)

const envelope: EmailEnvelope = {
  id: 'env-1',
  message_id: '<1@example.com>',
  account_id: 1,
  mailbox_id: 2,
  account_email: 'alice@example.com',
  mailbox_name: 'INBOX',
  uid: 7,
  subject: 'Invoice January',
  preview: 'Please find attached',
  from: 'billing@example.com',
  to: ['alice@example.com'],
  cc: [],
  bcc: [],
  date: DATE,
  internal_date: DATE,
  size: 2048,
  thread_id: 't-1',
  attachment_count: 0,
  regular_attachment_count: 0,
  tags: [],
  content_hash: 'h',
}

/** Serves one result and records every search request body. */
function serveResults(): Record<string, unknown>[] {
  const requests: Record<string, unknown>[] = []
  server.use(
    http.post(`${BASE}/api/v1/search-messages`, async ({ request }) => {
      requests.push((await request.json()) as Record<string, unknown>)
      return HttpResponse.json({
        current_page: 1,
        page_size: 20,
        total_items: 1,
        total_pages: 1,
        items: [envelope],
      })
    }),
    http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json([])),
    http.get(`${BASE}/api/v1/all-tags`, () => HttpResponse.json([])),
  )
  return requests
}

/** The URL search after every navigation the page asked for. */
function navigatedSearch(): Search {
  const calls = route.navigate.mock.calls as [{ search: (prev: Search) => Search }][]
  return calls.reduce<Search>((prev, [{ search }]) => search(prev), route.search)
}

describe('search page sorting', () => {
  beforeEach(() => {
    localStorage.clear()
    route.navigate.mockReset()
    route.search = { page: 1, pageSize: 20, sortBy: 'DATE', sortOrder: 'desc' }
  })

  it('asks the server for the sort in the URL', async () => {
    route.search = { ...route.search, sortBy: 'SIZE', sortOrder: 'asc' }
    const requests = serveResults()
    renderWithProviders(<EmailSearch />)

    expect(await screen.findByText('Invoice January')).toBeInTheDocument()
    expect(requests[0]).toMatchObject({ sort_by: 'SIZE', desc: false })
  })

  it('puts a column sort into the URL', async () => {
    serveResults()
    renderWithProviders(<EmailSearch />)
    await screen.findByText('Invoice January')

    await userEvent.click(screen.getByRole('button', { name: /^Size/ }))
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Asc' }))

    await waitFor(() => expect(navigatedSearch()).toMatchObject({ sortBy: 'SIZE', sortOrder: 'asc' }))
  })

  it('shows absolute dates when chosen', async () => {
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'absolute')
    serveResults()
    renderWithProviders(<EmailSearch />)

    expect(await screen.findByText(/2024/)).toBeInTheDocument()
  })
})
