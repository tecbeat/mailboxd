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
import { Proxy } from '@/api/system/api'

interface DataTableProps {
  columns: ColumnDef<DataTableFeatures, Proxy>[]
  data: Proxy[]
}

export function ProxyTable({ columns, data }: DataTableProps) {
  return (
    <DataTable
      columns={columns}
      data={data}
      storageKey='proxy'
      containerClassName='overflow-x-auto rounded-md border'
      tableClassName='w-full table-fixed'
      cellClassName='overflow-hidden'
      showPagination={data.length > 10}
      toolbar={(table) => <DataTableToolbar table={table} />}
    />
  )
}
