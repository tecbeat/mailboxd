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

import { afterEach, describe, it, expect } from 'vitest'
import { type ComponentProps } from 'react'
import userEvent from '@testing-library/user-event'
import { render, screen } from '@/test/test-utils'
import { DATE_DISPLAY_STORAGE_KEY } from '@/hooks/use-date-display'
import { DataTableViewOptions } from '../view-options'

type TableProp = ComponentProps<typeof DataTableViewOptions>['table']

describe('DataTableViewOptions', () => {
  afterEach(() => localStorage.clear())

  it('renders the column-visibility trigger button', () => {
    const table = { getAllColumns: () => [] } as unknown as TableProp
    render(<DataTableViewOptions table={table} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('switches the date display between relative and absolute', async () => {
    const table = { getAllColumns: () => [] } as unknown as TableProp
    render(<DataTableViewOptions table={table} />)

    await userEvent.click(screen.getByRole('button'))
    expect(await screen.findByRole('menuitemradio', { name: 'Relative' })).toBeChecked()

    await userEvent.click(screen.getByRole('menuitemradio', { name: 'Absolute' }))
    expect(localStorage.getItem(DATE_DISPLAY_STORAGE_KEY)).toBe('absolute')

    await userEvent.click(screen.getByRole('button'))
    expect(await screen.findByRole('menuitemradio', { name: 'Absolute' })).toBeChecked()
  })
})
