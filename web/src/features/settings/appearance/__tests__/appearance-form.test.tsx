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
import { DATE_FORMAT_STORAGE_KEY } from '@/hooks/use-date-format'

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

describe('AppearanceForm date format', () => {
  afterEach(() => localStorage.clear())

  it('stores the chosen format locally and sends only theme and language', async () => {
    let body: unknown
    server.use(
      http.post('http://localhost:15630/api/v1/users/1', async ({ request }) => {
        body = await request.json()
        return HttpResponse.json(null)
      }),
    )
    const { queryClient } = renderWithProviders(<AppearanceForm />)
    queryClient.setQueryData(['current-user'], { id: 1 })

    await userEvent.click(screen.getByRole('radio', { name: 'ISO 8601 (2026-01-31)' }))
    await userEvent.click(screen.getByRole('button', { name: 'Update preferences' }))

    await waitFor(() => expect(body).toBeDefined())
    expect(localStorage.getItem(DATE_FORMAT_STORAGE_KEY)).toBe('iso')
    expect(Object.keys(body as object).sort()).toEqual(['language', 'theme'])
  })

  it('preselects the stored format', () => {
    localStorage.setItem(DATE_FORMAT_STORAGE_KEY, 'iso')
    renderWithProviders(<AppearanceForm />)

    expect(screen.getByRole('radio', { name: 'ISO 8601 (2026-01-31)' })).toBeChecked()
    expect(screen.getByRole('radio', { name: /^Language default/ })).not.toBeChecked()
  })
})
