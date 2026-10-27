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

import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { BlobRepairRecord } from '@/api/repair/api'
import { DATE_DISPLAY_STORAGE_KEY } from '@/hooks/use-date-display'
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

import { RepairHistoryButton } from '../repair-history-button'
import { RepairHistoryDrawer } from '../repair-history-drawer'
import { repairStatusLabel, repairStatusVariant } from '../status'

const { permission } = vi.hoisted(() => ({ permission: vi.fn() }))
vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({ require_any_permission: permission }),
}))

const BASE = 'http://localhost:15630'

const record: BlobRepairRecord = {
  id: 'r-1',
  created_at: Date.UTC(2026, 9, 20, 8, 15),
  operator_id: 5,
  operator: 'alice',
  account_id: 3,
  mailbox_id: 2,
  subject: 'Invoice January',
  message_id: '<1@example.com>',
  report: {
    envelope_id: 'env-1',
    account_id: 3,
    mailbox_id: 2,
    subject: 'Invoice January',
    message_id: '<1@example.com>',
    status: 'partial',
    email_missing_before: false,
    email_missing_after: false,
    missing_before: ['hash-a'],
    missing_after: ['hash-a'],
    error: null,
  },
}

/** Serves `records` as the history and records each request's query string. */
function serveHistory(records: BlobRepairRecord[]): URLSearchParams[] {
  const queries: URLSearchParams[] = []
  server.use(
    http.get(`${BASE}/api/v1/repair-reports`, ({ request }) => {
      queries.push(new URL(request.url).searchParams)
      return HttpResponse.json(records)
    }),
    http.get(`${BASE}/api/v1/minimal-account-list`, () =>
      HttpResponse.json([{ id: 3, email: 'alice@example.com' }]),
    ),
  )
  return queries
}

describe('RepairHistoryDrawer', () => {
  beforeEach(() => localStorage.clear())

  it('lists the most recent repairs', async () => {
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'absolute')
    const queries = serveHistory([
      record,
      {
        ...record,
        id: 'r-2',
        subject: '',
        account_id: 9,
        operator: 'bob',
        report: { ...record.report, status: 'repaired' },
      },
    ])
    renderWithProviders(<RepairHistoryDrawer open onOpenChange={vi.fn()} />)

    expect(await screen.findByText('Invoice January')).toBeInTheDocument()
    expect(queries.map((q) => q.toString())).toEqual(['limit=200'])
    expect(screen.getByText('The most recent 200 repair attempts.')).toBeInTheDocument()

    const [, first, second] = screen.getAllByRole('row')
    expect(within(first).getByText('alice')).toBeInTheDocument()
    expect(within(first).getByText(/2026/)).toBeInTheDocument()
    expect(await within(first).findByText('alice@example.com')).toBeInTheDocument()
    expect(within(first).getByText('Partially repaired')).toBeInTheDocument()
    expect(within(second).getByText('—')).toBeInTheDocument()
    expect(within(second).getByText('9')).toBeInTheDocument()
    expect(within(second).getByText('Repaired')).toBeInTheDocument()
  })

  it('expands a row into its report', async () => {
    serveHistory([record])
    renderWithProviders(<RepairHistoryDrawer open onOpenChange={vi.fn()} />)

    await userEvent.click(await screen.findByRole('button', { name: 'Show details' }))

    const after = screen.getByRole('region', { name: 'Attachments still missing' })
    expect(within(after).getByText('hash-a')).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: 'Hide details' }))
    expect(screen.queryByRole('region', { name: 'Attachments still missing' })).not.toBeInTheDocument()
  })

  it('says when there are no repairs', async () => {
    serveHistory([])
    renderWithProviders(<RepairHistoryDrawer open onOpenChange={vi.fn()} />)

    expect(screen.getByRole('status', { name: 'Loading...' })).toBeInTheDocument()
    expect(await screen.findByText('No repairs yet.')).toBeInTheDocument()
  })

  it('says when the history cannot be loaded', async () => {
    server.use(
      http.get(`${BASE}/api/v1/repair-reports`, () => new HttpResponse(null, { status: 500 })),
      http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json([])),
    )
    renderWithProviders(<RepairHistoryDrawer open onOpenChange={vi.fn()} />)

    expect(await screen.findByText('Failed to load the repair history.')).toBeInTheDocument()
  })

  it('does not load the history while closed', async () => {
    const queries = serveHistory([record])
    renderWithProviders(<RepairHistoryDrawer open={false} onOpenChange={vi.fn()} />)

    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(queries).toEqual([])
  })
})

describe('RepairHistoryButton', () => {
  beforeEach(() => permission.mockReset())

  it('is hidden without the repair permission', () => {
    permission.mockReturnValue(false)
    renderWithProviders(<RepairHistoryButton />)

    expect(screen.queryByRole('button', { name: 'Repair history' })).not.toBeInTheDocument()
    expect(permission).toHaveBeenCalledWith(['system:root', 'data:manage'])
  })

  it('opens the history', async () => {
    permission.mockReturnValue(true)
    serveHistory([record])
    renderWithProviders(<RepairHistoryButton />)

    await userEvent.click(screen.getByRole('button', { name: 'Repair history' }))

    expect(await screen.findByRole('dialog', { name: 'Repair history' })).toBeInTheDocument()
    expect(await screen.findByText('Invoice January')).toBeInTheDocument()
  })
})

describe('repair status', () => {
  const t = ((key: string) => key) as unknown as Parameters<typeof repairStatusLabel>[0]

  it('shows an unknown status as is', () => {
    const status = 'quarantined' as never
    expect(repairStatusLabel(t, status)).toBe('quarantined')
    expect(repairStatusVariant(status)).toBe('outline')
  })
})
