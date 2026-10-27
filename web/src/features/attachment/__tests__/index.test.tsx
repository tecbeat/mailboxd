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

import type { AttachmentModel } from '@/api/attachment/api'
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

import AttachmentSearch from '../index'

const BASE = 'http://localhost:15630'
const DATE = Date.UTC(2024, 0, 15, 9, 30)

const attachment: AttachmentModel = {
  id: 'att-1',
  envelope_id: 'env-1',
  account_id: 1,
  account_email: 'alice@example.com',
  mailbox_id: 2,
  mailbox_name: 'INBOX',
  subject: 'Invoice January',
  content_hash: 'h',
  from: 'billing@example.com',
  date: DATE,
  ingest_at: DATE,
  size: 2048,
  ext: 'pdf',
  category: 'Document',
  content_type: 'application/pdf',
  shard_id: 0,
  has_text: true,
  is_ocr: false,
  is_indexed: true,
  is_message: false,
  name: 'invoice-january.pdf',
}

/** Serves one result and records every search request body. */
function serveResults(): Record<string, unknown>[] {
  const requests: Record<string, unknown>[] = []
  server.use(
    http.post(`${BASE}/api/v1/search-attachment`, async ({ request }) => {
      requests.push((await request.json()) as Record<string, unknown>)
      return HttpResponse.json({
        current_page: 1,
        page_size: 20,
        total_items: 1,
        total_pages: 1,
        items: [attachment],
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

describe('attachment page sorting', () => {
  beforeEach(() => {
    localStorage.clear()
    route.navigate.mockReset()
    route.search = { page: 1, pageSize: 20, sortBy: 'DATE', sortOrder: 'desc' }
  })

  it('sorts a text search by relevance from the URL and back to date', async () => {
    route.search = {
      ...route.search,
      sortBy: 'RELEVANCE',
      q: JSON.stringify({ text: 'invoice' }),
    }
    const requests = serveResults()
    renderWithProviders(<AttachmentSearch />)

    expect(await screen.findByText('invoice-january.pdf')).toBeInTheDocument()
    expect(requests[0]).toMatchObject({ sort_by: 'RELEVANCE' })
    const relevance = screen.getByRole('button', { name: /Relevance/ })
    expect(relevance).toHaveAttribute('aria-pressed', 'true')

    await userEvent.click(relevance)

    await waitFor(() => expect(navigatedSearch()).toMatchObject({ sortBy: 'DATE', sortOrder: 'desc' }))
  })

  it('shows absolute dates when chosen', async () => {
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'absolute')
    serveResults()
    renderWithProviders(<AttachmentSearch />)

    expect(await screen.findByText(/2024/)).toBeInTheDocument()
  })
})
