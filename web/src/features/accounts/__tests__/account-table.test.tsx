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

import { QueryClient } from '@tanstack/react-query'
import type { ColumnDef } from '@tanstack/react-table'
import { act, screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { AccountModel } from '@/api/account/api'
import type { DataTableFeatures } from '@/lib/data-table'
import { renderWithProviders } from '@/test/utils'

import { AccountTable } from '../components/table'

const columns: ColumnDef<DataTableFeatures, AccountModel>[] = [
  { accessorKey: 'id', header: 'ID column' },
  { accessorKey: 'account_name', header: 'Name column' },
  { accessorKey: 'email', header: 'Email column' },
  { id: 'actions', header: 'Actions column' },
]

const headers = () => screen.getAllByRole('columnheader').map((header) => header.textContent)

const account = { id: 7, account_name: 'Work', email: 'work@example.com' } as AccountModel

function renderTable(hidden?: string[], order?: string[]) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  queryClient.setQueryData(['current-user'], {
    id: 1,
    hidden_columns: hidden ? { accounts: hidden } : undefined,
    column_order: order ? { accounts: order } : undefined,
  })
  renderWithProviders(<AccountTable columns={columns} data={[account]} />, { queryClient })
  return queryClient
}

describe('AccountTable', () => {
  it('shows every column when the profile hides none', () => {
    renderTable()
    expect(screen.getByRole('columnheader', { name: 'ID column' })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Name column' })).toBeInTheDocument()
  })

  it('hides the columns stored on the profile', () => {
    renderTable(['id', 'email'])
    expect(screen.queryByRole('columnheader', { name: 'ID column' })).not.toBeInTheDocument()
    expect(screen.queryByText('7')).not.toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Name column' })).toBeInTheDocument()
    // Email is not hideable, so a stale entry for it is ignored.
    expect(screen.getByRole('columnheader', { name: 'Email column' })).toBeInTheDocument()
  })

  it('shows the columns in definition order by default', () => {
    renderTable()
    expect(headers()).toEqual(['ID column', 'Name column', 'Email column', 'Actions column'])
  })

  it('shows the columns in the order of the profile, with actions last', () => {
    renderTable(undefined, ['email', 'id'])
    expect(headers()).toEqual(['Email column', 'ID column', 'Name column', 'Actions column'])
  })

  it('combines the stored order with hidden columns', () => {
    renderTable(['id'], ['email', 'id', 'account_name'])
    expect(headers()).toEqual(['Email column', 'Name column', 'Actions column'])
  })

  it('follows a new order without a reload', async () => {
    const queryClient = renderTable()
    act(() => {
      queryClient.setQueryData(['current-user'], { id: 1, column_order: { accounts: ['account_name'] } })
    })
    await waitFor(() => expect(headers()[0]).toBe('Name column'))
  })

  it('follows profile updates without a reload', async () => {
    const queryClient = renderTable(['id'])
    act(() => {
      queryClient.setQueryData(['current-user'], { id: 1, hidden_columns: {} })
    })
    expect(await screen.findByRole('columnheader', { name: 'ID column' })).toBeInTheDocument()
  })
})
