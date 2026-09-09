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
import type { TFunction } from 'i18next'
import { type DataTableFeatures } from '@/lib/data-table'
import LongText from '@/components/long-text'
import { Badge } from '@/components/ui/badge'
import { DataTableColumnHeader } from '@/components/data-table/data-table-column-header'
import { formatDateTime } from '@/lib/utils'
import { AuditEntry } from '@/api/audit/api'

export const getColumns = (
  t: TFunction,
): ColumnDef<DataTableFeatures, AuditEntry>[] => [
  {
    accessorKey: 'created_at',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.timestamp', 'Time')}
      />
    ),
    cell: ({ row }) => <LongText>{formatDateTime(row.original.created_at)}</LongText>,
    enableHiding: false,
    enableSorting: false,
    meta: { className: 'w-44 whitespace-nowrap' },
  },
  {
    accessorKey: 'event_type',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.event', 'Event')}
      />
    ),
    cell: ({ row }) => (
      <Badge variant="outline" className="font-mono">
        {row.original.event_type}
      </Badge>
    ),
    enableSorting: false,
    meta: { className: 'w-48 whitespace-nowrap' },
  },
  {
    accessorKey: 'actor',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.actor', 'Actor')}
      />
    ),
    cell: ({ row }) => <LongText className="max-w-48">{row.original.actor}</LongText>,
    enableSorting: false,
    meta: { className: 'w-48' },
  },
  {
    accessorKey: 'target',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.target', 'Target')}
      />
    ),
    cell: ({ row }) => {
      const target = row.original.target
      return target ? (
        <LongText className="max-w-full" contentClassName="max-w-[24rem] break-all">
          {target}
        </LongText>
      ) : (
        <span className="text-muted-foreground">—</span>
      )
    },
    enableSorting: false,
    meta: { className: 'min-w-0' },
  },
  {
    accessorKey: 'detail',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.detail', 'Detail')}
      />
    ),
    cell: ({ row }) => {
      const detail = row.original.detail
      return detail ? (
        <LongText className="max-w-full" contentClassName="max-w-[24rem] break-all">
          {detail}
        </LongText>
      ) : (
        <span className="text-muted-foreground">—</span>
      )
    },
    enableSorting: false,
    meta: { className: 'min-w-0' },
  },
  {
    accessorKey: 'ip',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.ip', 'IP')}
      />
    ),
    cell: ({ row }) => {
      const ip = row.original.ip
      return ip ? (
        <LongText className="font-mono">{ip}</LongText>
      ) : (
        <span className="text-muted-foreground">—</span>
      )
    },
    enableSorting: false,
    meta: { className: 'w-36 whitespace-nowrap' },
  },
  {
    accessorKey: 'success',
    header: ({ column }) => (
      <DataTableColumnHeader
        column={column}
        title={t('auditLog.columns.status', 'Status')}
      />
    ),
    // `success` is only meaningful for login attempts; other events are always
    // recorded on success, so a status badge there would be noise.
    cell: ({ row }) => {
      const { event_type, success } = row.original
      if (event_type !== 'user.login') {
        return <span className="text-muted-foreground">—</span>
      }
      return (
        <Badge variant={success ? 'secondary' : 'destructive'}>
          {success
            ? t('auditLog.status.success', 'Success')
            : t('auditLog.status.failed', 'Failed')}
        </Badge>
      )
    },
    enableSorting: false,
    meta: { className: 'w-28 whitespace-nowrap' },
  },
]
