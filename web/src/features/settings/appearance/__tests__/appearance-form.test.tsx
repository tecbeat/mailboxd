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

import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { server } from '@/test/server'
import { mockListLayout, rowOrder } from '@/test/list-layout'
import { renderWithProviders } from '@/test/utils'
import {
  LEGACY_DATE_PATTERN_STORAGE_KEYS,
  readDatePattern,
  setDatePatterns,
} from '@/hooks/use-date-format'

import { AppearanceForm } from '../appearance-form'

// jsdom lacks the ResizeObserver the Radix radio group uses.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
)

const BASE = 'http://localhost:15630'

describe('AppearanceForm date format', () => {
  afterEach(() => setDatePatterns({ date: '', dateTime: '' }))

  it('saves the typed patterns to the profile', async () => {
    let body: unknown
    server.use(
      http.post(`${BASE}/api/v1/users/1`, async ({ request }) => {
        body = await request.json()
        return HttpResponse.json(null)
      }),
    )
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], { id: 1 })

    await userEvent.type(screen.getByRole('textbox', { name: 'Date' }), 'MM.YY')
    expect(screen.getByText('Example: 01.26')).toBeInTheDocument()
    await userEvent.type(screen.getByRole('textbox', { name: 'Date and time' }), 'MM.DD.YYYY HH:mm')
    expect(screen.getByText('Example: 01.31.2026 14:30')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(body).toBeDefined())
    expect(body).toMatchObject({ date_pattern: 'MM.YY', date_time_pattern: 'MM.DD.YYYY HH:mm' })
    expect(readDatePattern('date')).toBe('MM.YY')
    expect(readDatePattern('dateTime')).toBe('MM.DD.YYYY HH:mm')
    expect(localStorage.getItem(LEGACY_DATE_PATTERN_STORAGE_KEYS.date)).toBeNull()
    expect(localStorage.getItem(LEGACY_DATE_PATTERN_STORAGE_KEYS.dateTime)).toBeNull()
  })

  it('cannot be saved while a pattern is invalid', async () => {
    renderWithProviders(<AppearanceForm />)

    await userEvent.type(screen.getByRole('textbox', { name: 'Date' }), 'MM.JJ')

    expect(await screen.findByText(/^Invalid format/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Update preferences' })).toBeDisabled()

    await userEvent.clear(screen.getByRole('textbox', { name: 'Date' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Update preferences' })).toBeEnabled())
    expect(screen.getAllByText(/^Language default: /)).toHaveLength(2)
  })

  it('fills in the patterns of the profile', () => {
    setDatePatterns({ date: 'YYYY-MM-DD', dateTime: '' })
    renderWithProviders(<AppearanceForm />)

    expect(screen.getByRole('textbox', { name: 'Date' })).toHaveValue('YYYY-MM-DD')
    expect(screen.getByRole('textbox', { name: 'Date and time' })).toHaveValue('')
    expect(screen.getByText('Example: 2026-01-31')).toBeInTheDocument()
  })

  it('takes over patterns that arrive after the form opened', () => {
    renderWithProviders(<AppearanceForm />)
    act(() => setDatePatterns({ date: 'YYYY-MM-DD', dateTime: '' }))

    expect(screen.getByRole('textbox', { name: 'Date' })).toHaveValue('YYYY-MM-DD')
  })
})

describe('AppearanceForm account table columns', () => {
  let restoreLayout: () => void
  beforeEach(() => {
    restoreLayout = mockListLayout()
  })
  afterEach(() => restoreLayout())

  function captureUpdate() {
    const request: { body?: { hidden_columns?: unknown; column_order?: unknown } } = {}
    server.use(
      http.post(`${BASE}/api/v1/users/1`, async ({ request: req }) => {
        request.body = (await req.json()) as typeof request.body
        return HttpResponse.json(null)
      }),
    )
    return request
  }

  it('shows the stored order and columns once the profile has loaded', async () => {
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], {
      id: 1,
      hidden_columns: { accounts: ['id'] },
      column_order: { accounts: ['updated_at', 'email'] },
    })

    await waitFor(() => expect(rowOrder().slice(0, 2)).toEqual(['updated_at', 'email']))
    expect(rowOrder()).toHaveLength(11)
    expect(screen.getByRole('switch', { name: 'Show ID' })).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Show Name' })).toBeChecked()
  })

  it('saves the switched off columns and the new order to the profile', async () => {
    const update = captureUpdate()
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], { id: 1, hidden_columns: { accounts: ['id'] } })
    await waitFor(() => expect(screen.getByRole('switch', { name: 'Show ID' })).not.toBeChecked())

    await userEvent.click(screen.getByRole('switch', { name: 'Show ID' }))
    await userEvent.click(screen.getByRole('switch', { name: 'Show Name' }))
    screen.getByRole('button', { name: 'Move Updated At' }).focus()
    await userEvent.keyboard('[Space]')
    for (let i = 0; i < 10; i++) await userEvent.keyboard('[ArrowUp]')
    await userEvent.keyboard('[Space]')
    await userEvent.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(update.body).toBeDefined())
    expect(update.body?.hidden_columns).toEqual({ accounts: ['account_name'] })
    expect((update.body?.column_order as { accounts: string[] }).accounts.slice(0, 2)).toEqual(['updated_at', 'id'])
  })

  it('saves the default columns after a reset', async () => {
    const update = captureUpdate()
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], {
      id: 1,
      hidden_columns: { accounts: ['id', 'created_by'] },
      column_order: { accounts: ['updated_at'] },
    })
    await waitFor(() => expect(rowOrder()[0]).toBe('updated_at'))

    await userEvent.click(screen.getByRole('button', { name: 'Restore default columns' }))
    await userEvent.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(update.body).toBeDefined())
    expect(update.body?.hidden_columns).toEqual({ accounts: [] })
    expect(update.body?.column_order).toEqual({ accounts: [] })
  })
})
