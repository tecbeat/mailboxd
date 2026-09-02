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

import { describe, it, expect } from 'vitest'
import { type ComponentProps } from 'react'
import { render, screen } from '@/test/test-utils'
import { DataTableViewOptions } from '../view-options'

type TableProp = ComponentProps<typeof DataTableViewOptions>['table']

describe('DataTableViewOptions', () => {
  it('renders the column-visibility trigger button', () => {
    const table = { getAllColumns: () => [] } as unknown as TableProp
    render(<DataTableViewOptions table={table} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })
})
