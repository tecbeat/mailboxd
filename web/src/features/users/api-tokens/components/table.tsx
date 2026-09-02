//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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


import { ColumnDef } from '@tanstack/react-table'
import { type DataTableFeatures } from '@/lib/data-table'
import { DataTable } from '@/components/data-table/data-table'
import { DataTableToolbar } from './data-table-toolbar'
import { AccessToken } from '@/api/users/api'

interface DataTableProps {
  columns: ColumnDef<DataTableFeatures, AccessToken>[]
  data: AccessToken[]
}

export function ApiTokensTable({ columns, data }: DataTableProps) {
  return (
    <DataTable
      columns={columns}
      data={data}
      storageKey='apitoken'
      pageSizeOptions={[10, 20, 30, 40, 50]}
      globalFilterFn={(row, _, filterValue) => {
        const searchValue = filterValue.toLowerCase()
        const name = row.original.name?.toLowerCase() ?? ''
        const owner = row.original.user_name?.toLowerCase() ?? ''
        const email = row.original.user_email?.toLowerCase() ?? ''
        const token = row.original.token?.toLowerCase() ?? ''
        return (
          name.includes(searchValue) ||
          owner.includes(searchValue) ||
          email.includes(searchValue) ||
          token.includes(searchValue)
        )
      }}
      toolbar={(table) => <DataTableToolbar table={table} />}
    />
  )
}
