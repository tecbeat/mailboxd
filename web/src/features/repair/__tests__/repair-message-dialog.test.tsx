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

import { RepairMessageDialog } from '../repair-message-dialog'

const { toast } = vi.hoisted(() => ({ toast: vi.fn() }))
vi.mock('@/hooks/use-toast', () => ({ toast }))

const BASE = 'http://localhost:15630'

function makeReport(overrides: Partial<BlobRepairReport> = {}): BlobRepairReport {
  return {
    envelope_id: 'env-1',
    account_id: 3,
    mailbox_id: 2,
    subject: 'Invoice January',
    message_id: '<1@example.com>',
    status: 'complete',
    email_missing_before: false,
    email_missing_after: false,
    missing_before: [],
    missing_after: [],
    error: null,
    ...overrides,
  }
}

/** Serves `report` for the repair endpoint and records the request paths. */
function serveRepair(report: BlobRepairReport): string[] {
  const paths: string[] = []
  server.use(
    http.post(`${BASE}/api/v1/repair-message/:account/:envelope`, ({ request }) => {
      paths.push(new URL(request.url).pathname)
      return HttpResponse.json(report)
    }),
  )
  return paths
}

function renderDialog(props: Partial<Parameters<typeof RepairMessageDialog>[0]> = {}) {
  const onOpenChange = vi.fn()
  const result = renderWithProviders(
    <RepairMessageDialog
      open
      onOpenChange={onOpenChange}
      accountId={3}
      envelopeId='env-1'
      subject='Invoice January'
      {...props}
    />,
  )
  return { ...result, onOpenChange }
}

async function runRepair() {
  await userEvent.click(screen.getByRole('button', { name: 'Repair' }))
  return screen.findByRole('heading', { name: 'Repair report' })
}

describe('RepairMessageDialog', () => {
  beforeEach(() => toast.mockReset())

  it('asks for confirmation before repairing', () => {
    renderDialog()

    expect(screen.getByRole('heading', { name: 'Repair message' })).toBeInTheDocument()
    expect(screen.getByText('Invoice January')).toBeInTheDocument()
    expect(screen.getByText(/whole message/)).toBeInTheDocument()
  })

  it('posts the repair for the account and envelope', async () => {
    const paths = serveRepair(makeReport())
    renderDialog()

    await runRepair()

    expect(paths).toEqual(['/api/v1/repair-message/3/env-1'])
  })

  it('reports that nothing was missing', async () => {
    serveRepair(makeReport({ status: 'complete', subject: '', message_id: '' }))
    renderDialog({ subject: 'Draft' })

    await runRepair()

    expect(screen.getByText('Nothing was missing')).toBeInTheDocument()
    expect(screen.queryByText('Draft')).not.toBeInTheDocument()
    expect(screen.queryByText(/Attachments/)).not.toBeInTheDocument()
    expect(screen.queryByText('Error')).not.toBeInTheDocument()
  })

  it('reports a repair with the restored content', async () => {
    serveRepair(
      makeReport({
        status: 'repaired',
        email_missing_before: true,
        missing_before: ['hash-a', 'hash-b'],
      }),
    )
    renderDialog()

    await runRepair()

    expect(screen.getByText('Repaired')).toBeInTheDocument()
    expect(screen.getByText('<1@example.com>')).toBeInTheDocument()
    expect(screen.getByText('Message content restored')).toBeInTheDocument()
    const before = screen.getByRole('region', { name: 'Attachments missing before the repair' })
    expect(within(before).getAllByRole('listitem').map((li) => li.textContent)).toEqual([
      'hash-a',
      'hash-b',
    ])
    expect(screen.queryByRole('region', { name: 'Attachments still missing' })).not.toBeInTheDocument()
  })

  it('names what is still missing after a partial repair', async () => {
    serveRepair(
      makeReport({
        status: 'partial',
        email_missing_before: true,
        email_missing_after: true,
        missing_before: ['hash-a'],
        missing_after: ['hash-a'],
      }),
    )
    renderDialog()

    await runRepair()

    expect(screen.getByText('Partially repaired')).toBeInTheDocument()
    expect(screen.getByText('Message content is still missing')).toBeInTheDocument()
    expect(screen.queryByText('Message content restored')).not.toBeInTheDocument()
    expect(screen.getByText(/never stored/)).toBeInTheDocument()
    const after = screen.getByRole('region', { name: 'Attachments still missing' })
    expect(within(after).getByText('hash-a')).toBeInTheDocument()
  })

  it('shows the error of a failed repair', async () => {
    serveRepair(makeReport({ status: 'failed', error: 'IMAP fetch timed out' }))
    renderDialog()

    await runRepair()

    expect(screen.getByText('Failed')).toBeInTheDocument()
    expect(screen.getByText(/could not be fetched or verified/)).toBeInTheDocument()
    expect(screen.getByText('Error')).toBeInTheDocument()
    expect(screen.getByText('IMAP fetch timed out')).toBeInTheDocument()
    expect(toast).not.toHaveBeenCalled()
  })

  it('shows progress while the repair runs', async () => {
    let release!: () => void
    const gate = new Promise<void>((resolve) => (release = resolve))
    server.use(
      http.post(`${BASE}/api/v1/repair-message/:account/:envelope`, async () => {
        await gate
        return HttpResponse.json(makeReport())
      }),
    )
    renderDialog()

    await userEvent.click(screen.getByRole('button', { name: 'Repair' }))

    expect(await screen.findByRole('status')).toHaveTextContent('Repairing…')
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Repair' })).toBeDisabled()
    release()
    await screen.findByRole('heading', { name: 'Repair report' })
  })

  it('refreshes the repair history after a recorded repair', async () => {
    serveRepair(makeReport({ status: 'repaired' }))
    const { queryClient } = renderDialog()
    queryClient.setQueryData(['repair-reports', 200], [])

    await runRepair()

    await waitFor(() =>
      expect(queryClient.getQueryState(['repair-reports', 200])?.isInvalidated).toBe(true),
    )
  })

  it('leaves the repair history alone when nothing was missing', async () => {
    serveRepair(makeReport({ status: 'complete' }))
    const { queryClient } = renderDialog()
    queryClient.setQueryData(['repair-reports', 200], [])

    await runRepair()

    expect(queryClient.getQueryState(['repair-reports', 200])?.isInvalidated).toBe(false)
  })

  it('toasts the server message when the request is rejected', async () => {
    server.use(
      http.post(`${BASE}/api/v1/repair-message/:account/:envelope`, () =>
        HttpResponse.json({ message: 'Permission denied' }, { status: 403 }),
      ),
    )
    renderDialog()

    await userEvent.click(screen.getByRole('button', { name: 'Repair' }))

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: 'destructive',
          title: 'Repair request failed',
          description: 'Permission denied',
        }),
      ),
    )
    expect(screen.getByRole('button', { name: 'Repair' })).toBeEnabled()
  })

  it('falls back to the transport error without a server message', async () => {
    server.use(
      http.post(`${BASE}/api/v1/repair-message/:account/:envelope`, () => HttpResponse.error()),
    )
    renderDialog()

    await userEvent.click(screen.getByRole('button', { name: 'Repair' }))

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(expect.objectContaining({ description: 'Network Error' })),
    )
  })

  it('cannot repair without a target message', () => {
    renderDialog({ envelopeId: undefined, subject: undefined })

    expect(screen.getByRole('button', { name: 'Repair' })).toBeDisabled()
    expect(screen.getByText('—')).toBeInTheDocument()
  })

  it('closes from the confirmation and from the report', async () => {
    serveRepair(makeReport())
    const { onOpenChange } = renderDialog()

    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onOpenChange).toHaveBeenLastCalledWith(false)

    onOpenChange.mockClear()
    await runRepair()
    const closeButtons = screen.getAllByRole('button', { name: 'Close' })
    await userEvent.click(closeButtons[closeButtons.length - 1])
    expect(onOpenChange).toHaveBeenLastCalledWith(false)
  })

  it('starts over when reopened', async () => {
    serveRepair(makeReport())
    const { rerender } = renderDialog()
    await runRepair()

    rerender(
      <RepairMessageDialog open={false} onOpenChange={vi.fn()} accountId={3} envelopeId='env-1' />,
    )
    rerender(
      <RepairMessageDialog open onOpenChange={vi.fn()} accountId={3} envelopeId='env-1' />,
    )

    expect(screen.getByRole('heading', { name: 'Repair message' })).toBeInTheDocument()
  })
})
