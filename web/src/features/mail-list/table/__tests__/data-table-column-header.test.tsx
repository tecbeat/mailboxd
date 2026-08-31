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

import { describe, it, expect, vi } from 'vitest'
import { type ComponentProps } from 'react'
import { render, screen } from '@/test/test-utils'
import { DataTableColumnHeader } from '../data-table-column-header'

type ColumnProp = ComponentProps<typeof DataTableColumnHeader>['column']

describe('DataTableColumnHeader', () => {
  it('renders a plain title when the column cannot be sorted', () => {
    const column = { getCanSort: () => false } as unknown as ColumnProp
    render(<DataTableColumnHeader column={column} title="Subject" />)
    expect(screen.getByText('Subject')).toBeInTheDocument()
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('renders a sort trigger button when the column can be sorted', () => {
    const column = {
      getCanSort: () => true,
      getIsSorted: () => false,
      toggleSorting: vi.fn(),
    } as unknown as ColumnProp
    render(<DataTableColumnHeader column={column} title="Date" />)
    expect(screen.getByRole('button')).toHaveTextContent('Date')
  })
})
