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

import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { BlobRepairReport } from '@/api/repair/api'
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

const { permission, toast } = vi.hoisted(() => ({ permission: vi.fn(), toast: vi.fn() }))

vi.mock('@tanstack/react-router', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-router')>()),
  getRouteApi: () => ({
    useSearch: () => ({ page: 1, pageSize: 20, sortBy: 'DATE', sortOrder: 'desc' }),
    useNavigate: () => vi.fn(),
  }),
  useNavigate: () => vi.fn(),
}))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@/hooks/use-toast', () => ({ toast }))
vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({ require_any_permission: permission }),
}))

import type { EmailEnvelope } from '@/api'
import EmailSearch from '../index'

const BASE = 'http://localhost:15630'
const DATE = Date.UTC(2024, 0, 15, 9, 30)

const item: EmailEnvelope = {
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

const report: BlobRepairReport = {
  envelope_id: 'env-1',
  account_id: 1,
  mailbox_id: 2,
  subject: 'Invoice January',
  message_id: '<1@example.com>',
  status: 'repaired',
  email_missing_before: true,
  email_missing_after: false,
  missing_before: [],
  missing_after: [],
  error: null,
}

/** Serves one result and records the repair request paths. */
function serve(): string[] {
  const repairs: string[] = []
  server.use(
    http.post(`${BASE}/api/v1/search-messages`, () =>
      HttpResponse.json({
        current_page: 1,
        page_size: 20,
        total_items: 1,
        total_pages: 1,
        items: [item],
      }),
    ),
    http.post(`${BASE}/api/v1/repair-message/:account/:envelope`, ({ request }) => {
      repairs.push(new URL(request.url).pathname)
      return HttpResponse.json(report)
    }),
    http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json([])),
    http.get(`${BASE}/api/v1/all-tags`, () => HttpResponse.json([])),
  )
  return repairs
}

async function openRowMenu() {
  const row = (await screen.findByText('Invoice January')).closest('tr')!
  await userEvent.click(within(row).getByRole('button', { name: 'Open menu' }))
  return screen.findByRole('menu')
}

describe('search repair', () => {
  beforeEach(() => {
    localStorage.clear()
    permission.mockReset()
  })

  it('hides the repair action and history without the permission', async () => {
    permission.mockReturnValue(false)
    serve()
    renderWithProviders(<EmailSearch />)

    const menu = await openRowMenu()

    expect(within(menu).queryByRole('menuitem', { name: /Repair message/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Repair history' })).not.toBeInTheDocument()
    expect(permission).toHaveBeenCalledWith(['system:root', 'data:manage'], 1)
  })

  it('repairs the message of a row', async () => {
    permission.mockReturnValue(true)
    const repairs = serve()
    renderWithProviders(<EmailSearch />)

    const menu = await openRowMenu()
    await userEvent.click(within(menu).getByRole('menuitem', { name: /Repair message/ }))
    const dialog = await screen.findByRole('dialog', { name: 'Repair message' })
    expect(within(dialog).getByText('Invoice January')).toBeInTheDocument()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Repair' }))

    expect(await screen.findByText('Message content restored')).toBeInTheDocument()
    expect(repairs).toEqual(['/api/v1/repair-message/1/env-1'])

    const closeButtons = within(dialog).getAllByRole('button', { name: 'Close' })
    await userEvent.click(closeButtons[closeButtons.length - 1])
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })

  it('offers the repair history in the toolbar', async () => {
    permission.mockReturnValue(true)
    serve()
    renderWithProviders(<EmailSearch />)

    expect(await screen.findByRole('button', { name: 'Repair history' })).toBeInTheDocument()
  })
})
