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

import { ColumnDef } from '@tanstack/react-table'
import { AuditEntry } from '@/api/audit/api'
import { type DataTableFeatures } from '@/lib/data-table'
import { DataTable } from '@/components/data-table/data-table'

interface DataTableProps {
  columns: ColumnDef<DataTableFeatures, AuditEntry>[]
  data: AuditEntry[]
}

export function AuditLogTable({ columns, data }: DataTableProps) {
  return (
    <DataTable
      columns={columns}
      data={data}
      storageKey="audit-log"
      containerClassName="w-full rounded-md border"
      tableClassName="w-full table-fixed"
      cellClassName="overflow-hidden"
      // Server-side pagination is driven by the controlled footer on the page,
      // so the table's own internal pagination stays off.
      showPagination={false}
    />
  )
}
