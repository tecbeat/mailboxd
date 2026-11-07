//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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

import { describe, expect, it } from 'vitest'

import { renderWithProviders } from '@/test/utils'
import { TABLE_COLUMNS } from '@/lib/table-columns'

import { useColumns } from '../components/columns'

describe('accounts column definitions', () => {
  it('match the movable columns of the settings, with actions last', () => {
    let ids: string[] = []
    function Probe() {
      ids = useColumns().map((column) => column.id ?? (column as { accessorKey?: string }).accessorKey ?? '')
      return null
    }
    renderWithProviders(<Probe />)
    expect(ids).toEqual([...TABLE_COLUMNS.accounts.map((column) => column.id), 'actions'])
  })
})
