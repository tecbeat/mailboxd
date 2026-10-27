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
import { NoSyncAccountDialog } from '../nosync-dialog'

vi.mock('@/hooks/use-toast', () => ({ toast: vi.fn() }))

const BASE = 'http://localhost:15630'

const account = (over: Partial<AccountModel> = {}): AccountModel => ({
  id: 5,
  account_type: 'NoSync',
  enabled: true,
  email: 'archive@example.com',
  download_folders: [],
  created_by: 1,
  created_user_name: 'admin',
  created_user_email: 'admin@example.com',
  created_at: 0,
  updated_at: 0,
  use_dangerous: false,
  ...over,
})

/** Capture the JSON body posted to `path`. */
function captureBody(path: string) {
  const sent: { body?: Record<string, unknown> } = {}
  server.use(
    http.post(`${BASE}/${path}`, async ({ request }) => {
      sent.body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json({})
    })
  )
  return sent
}

const days = () => screen.getByLabelText('Retention period (days)')

describe('NoSyncAccountDialog retention', () => {
  beforeAll(() => {
    // jsdom lacks the layout API the dialog's scroll area uses.
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      }
    )
  })

  it('creates an account with the chosen retention period', async () => {
    const sent = captureBody('api/v1/account')
    renderWithProviders(<NoSyncAccountDialog open onOpenChange={() => {}} />)

    await userEvent.type(screen.getByPlaceholderText(/@/), 'new@example.com')
    await userEvent.type(days(), '30')
    await userEvent.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(sent.body).toBeDefined())
    expect(sent.body).toMatchObject({
      email: 'new@example.com',
      account_type: 'NoSync',
      retention_days: 30,
    })
  })

  it('creates an account without retention when the field stays empty', async () => {
    const sent = captureBody('api/v1/account')
    renderWithProviders(<NoSyncAccountDialog open onOpenChange={() => {}} />)

    await userEvent.type(screen.getByPlaceholderText(/@/), 'keep@example.com')
    await userEvent.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(sent.body).toBeDefined())
    expect(sent.body).not.toHaveProperty('retention_days')
  })

  it('shows the stored period when editing', () => {
    renderWithProviders(
      <NoSyncAccountDialog
        open
        onOpenChange={() => {}}
        currentRow={account({ retention_days: 90 })}
      />
    )
    expect(days()).toHaveValue(90)
  })

  it('turns retention off with 0 when the field is cleared', async () => {
    const sent = captureBody('api/v1/account/5')
    renderWithProviders(
      <NoSyncAccountDialog
        open
        onOpenChange={() => {}}
        currentRow={account({ retention_days: 90 })}
      />
    )

    await userEvent.clear(days())
    await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(sent.body).toBeDefined())
    expect(sent.body).toMatchObject({ retention_days: 0 })
  })

  it('sends a changed period', async () => {
    const sent = captureBody('api/v1/account/5')
    renderWithProviders(
      <NoSyncAccountDialog
        open
        onOpenChange={() => {}}
        currentRow={account({ retention_days: null })}
      />
    )

    expect(days()).toHaveValue(null)
    await userEvent.type(days(), '7')
    await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(sent.body).toBeDefined())
    expect(sent.body).toMatchObject({ retention_days: 7 })
  })

  it('does not submit an out-of-range period', async () => {
    const sent = captureBody('api/v1/account/5')
    renderWithProviders(
      <NoSyncAccountDialog
        open
        onOpenChange={() => {}}
        currentRow={account()}
      />
    )

    await userEvent.type(days(), '36501')
    await userEvent.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(
      await screen.findByText(/whole number of days from 1 to 36500/)
    ).toBeInTheDocument()
    expect(sent.body).toBeUndefined()
  })
})
