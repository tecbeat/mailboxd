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
import { afterEach, describe, expect, it, vi } from 'vitest'

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'
import { DATE_PATTERN_STORAGE_KEYS } from '@/hooks/use-date-format'

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
  afterEach(() => localStorage.clear())

  it('stores the typed patterns locally and sends no patterns to the server', async () => {
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
    expect(localStorage.getItem(DATE_PATTERN_STORAGE_KEYS.date)).toBe('MM.YY')
    expect(localStorage.getItem(DATE_PATTERN_STORAGE_KEYS.dateTime)).toBe('MM.DD.YYYY HH:mm')
    expect(Object.keys(body as object).sort()).toEqual(['hidden_columns', 'language', 'theme'])
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

  it('fills in the stored patterns', () => {
    localStorage.setItem(DATE_PATTERN_STORAGE_KEYS.date, 'YYYY-MM-DD')
    renderWithProviders(<AppearanceForm />)

    expect(screen.getByRole('textbox', { name: 'Date' })).toHaveValue('YYYY-MM-DD')
    expect(screen.getByRole('textbox', { name: 'Date and time' })).toHaveValue('')
    expect(screen.getByText('Example: 2026-01-31')).toBeInTheDocument()
  })
})

describe('AppearanceForm account table columns', () => {
  it('shows the stored selection once the profile has loaded', async () => {
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], { id: 1, hidden_columns: { accounts: ['id'] } })

    await waitFor(() => expect(screen.getByRole('checkbox', { name: 'ID' })).not.toBeChecked())
    expect(screen.getByRole('checkbox', { name: 'Name' })).toBeChecked()
    expect(screen.queryByRole('checkbox', { name: 'Email' })).not.toBeInTheDocument()
  })

  it('saves the unchecked columns as hidden on the profile', async () => {
    let body: { hidden_columns?: unknown } | undefined
    server.use(
      http.post(`${BASE}/api/v1/users/1`, async ({ request }) => {
        body = (await request.json()) as typeof body
        return HttpResponse.json(null)
      }),
    )
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], { id: 1, hidden_columns: { accounts: ['id'] } })
    await waitFor(() => expect(screen.getByRole('checkbox', { name: 'ID' })).not.toBeChecked())

    await userEvent.click(screen.getByRole('checkbox', { name: 'ID' }))
    await userEvent.click(screen.getByRole('checkbox', { name: 'Name' }))
    await userEvent.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(body).toBeDefined())
    expect(body?.hidden_columns).toEqual({ accounts: ['account_name'] })
  })
})
