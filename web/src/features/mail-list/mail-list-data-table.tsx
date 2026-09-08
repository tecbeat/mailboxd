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
import {
  useState,
  useEffect,
  type ReactNode,
  type MouseEvent as ReactMouseEvent,
} from 'react'
import {
  ColumnDef,
  ColumnFiltersState,
  Row,
  Table,
  flexRender,
  useTable,
} from '@tanstack/react-table'
import { useTranslation } from 'react-i18next'
import { mailTableFeatures, type MailTableFeatures } from '@/lib/data-table'
import { cn } from '@/lib/utils'
import { Checkbox } from '@/components/ui/checkbox'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Table as ShadcnTable,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { useMailListConfig } from './config'

// Row entity requirements: every mail-list row is keyed by a string id scoped
// to a numeric account_id (the selection map is Map<account_id, Set<id>>).
export interface MailListRow {
  id: string
  account_id: number
}

interface MailListDataTableProps<TEntity extends MailListRow> {
  items: TEntity[]
  isLoading: boolean
  // True while a background refetch is in flight but previous results are still
  // shown (e.g. a new search term). Drives the subtle fade-while-updating on the
  // table body — distinct from `isLoading`, which is the empty first load.
  isFetching?: boolean
  // Feature-specific columns. The selection checkbox column is prepended here,
  // so features supply only their own middle/actions columns.
  columns: ColumnDef<MailTableFeatures, TEntity>[]
  setSortBy: (sortBy: 'DATE' | 'SIZE') => void
  setSortOrder: (value: 'desc' | 'asc') => void
  onRowClick: (
    e: ReactMouseEvent<HTMLTableRowElement, MouseEvent>,
    row: Row<MailTableFeatures, TEntity>
  ) => void
  // Feature toolbar rendered above the table, wired to the table instance.
  toolbar: (table: Table<MailTableFeatures, TEntity>) => ReactNode
  // Optional bulk-action bar shown while at least one row is selected.
  bulkActions?: ReactNode
}

// The shared mail-list table: owns selection state, the checkbox column, the
// loading skeleton, and the table render. Features parametrise it with their
// entity type, columns, toolbar and (optionally) a bulk-action bar.
export function MailListDataTable<TEntity extends MailListRow>({
  items,
  isLoading,
  isFetching = false,
  columns,
  setSortBy,
  setSortOrder,
  onRowClick,
  toolbar,
  bulkActions,
}: MailListDataTableProps<TEntity>) {
  const { t } = useTranslation()
  const { useListContext } = useMailListConfig()
  const { selected, setSelected, sorting, setSorting } = useListContext()
  const [rowSelection, setRowSelection] = useState({})
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([])

  const totalSelected = Array.from(selected.values()).reduce(
    (sum, set) => sum + set.size,
    0
  )

  const hasSelected = (accountId: number, mailId: string) =>
    selected.get(accountId)?.has(mailId) ?? false

  const handleToggleAll = () => {
    const total = Array.from(selected.values()).reduce(
      (sum, set) => sum + set.size,
      0
    )

    if (total === items.length && items.length > 0) {
      setSelected(new Map())
    } else {
      setSelected((prev) => {
        const next = new Map(prev)
        for (const item of items) {
          const set = new Set(next.get(item.account_id) || [])
          set.add(item.id)
          next.set(item.account_id, set)
        }
        return next
      })
    }
  }

  const toggleSelected = (accountId: number, mailId: string) => {
    setSelected((prev) => {
      const next = new Map(prev)
      const set = new Set(next.get(accountId) || [])

      if (set.has(mailId)) {
        set.delete(mailId)
        if (set.size === 0) next.delete(accountId)
        else next.set(accountId, set)
      } else {
        set.add(mailId)
        next.set(accountId, set)
      }
      return next
    })
  }

  const selectionColumn: ColumnDef<MailTableFeatures, TEntity> = {
    accessorKey: 'id',
    header: () => (
      <Checkbox
        checked={
          totalSelected === items.length && items.length > 0
            ? true
            : totalSelected > 0
              ? 'indeterminate'
              : false
        }
        onCheckedChange={handleToggleAll}
        className='h-4 w-4'
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={hasSelected(row.original.account_id, row.original.id)}
        onCheckedChange={() =>
          toggleSelected(row.original.account_id, row.original.id)
        }
        onClick={(e) => e.stopPropagation()}
        className='h-4 w-4 shrink-0'
      />
    ),
    meta: { className: 'text-left text-sm' },
    minSize: 25,
    maxSize: 25,
  }

  const allColumns = [selectionColumn, ...columns]

  useEffect(() => {
    const [value] = sorting
    setSortBy(value.id.toUpperCase() as 'DATE' | 'SIZE')
    setSortOrder(value.desc ? 'desc' : 'asc')
  }, [sorting, setSortBy, setSortOrder])

  const table = useTable({
    features: mailTableFeatures,
    data: items,
    columns: allColumns,
    state: {
      sorting,
      rowSelection,
      columnFilters,
    },
    enableRowSelection: true,
    onRowSelectionChange: setRowSelection,
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
  })

  const skeletonRows = (
    <div className='divide-y divide-border'>
      {Array.from({ length: 30 }).map((_, i) => (
        <div key={i} className='flex items-center gap-2 px-2 py-1.5'>
          <Skeleton className='h-3 w-3' />
          <Skeleton className='h-3 w-3 rounded-full' />
          <Skeleton className='h-3 flex-1' />
          <Skeleton className='h-2.5 w-16' />
        </div>
      ))}
    </div>
  )

  return (
    <>
      <div className='flex min-h-0 flex-col gap-0.5'>
        {/*
          The toolbar (which owns the search input) stays mounted while a query
          is in flight, so the search input never loses focus mid-type. With
          keepPreviousData on the search query, `isLoading` is true only on the
          empty first load (skeleton below); each subsequent keystroke keeps the
          previous rows and surfaces via `isFetching`, which just fades the table
          body — so only the table updates, not the whole page.
        */}
        {/*
          Reserve the vertical scrollbar's gutter beside the toolbar so the
          toolbar's right edge lines up with the bordered table below, which is
          inset by the OUTER's own stable gutter. overflow-hidden turns this
          into a scroll container so scrollbar-gutter:stable takes effect, and
          scrollbar-thin matches the OUTER's scrollbar metrics exactly — no bar
          is ever shown here. Without this, the full-width toolbar would overhang
          the table's right border by the scrollbar's width.
        */}
        <div className='overflow-hidden scrollbar-thin [scrollbar-gutter:stable]'>
          {toolbar(table)}
        </div>
        {/*
          Hug-content layout: this root is a normal flex child (flex: 0 1 auto),
          so it is only as tall as its rows and does NOT stretch — the external
          pagination sits directly under a short list instead of below an empty
          box. When the rows would exceed the available height, the root shrinks
          (min-h-0) and the OUTER region below (flex-1 min-h-0 overflow-auto)
          becomes the scroll container, so the pinned header and the external
          footer/pagination stay put. The sticky <th> row (see ui/table.tsx)
          then sticks to the top of THIS box rather than behind the app header.
          `relative` makes this box the containing block for any
          absolutely-positioned descendants (e.g. screen-reader-only labels), so
          their overflow stays contained here instead of inflating the app
          shell's scroll region.

          Two layers split scroll from border: the OUTER owns the scroll so BOTH
          scrollbars sit OUTSIDE the bordered box — the vertical bar in its own
          strip to the right, never over the rows. The INNER owns the border and
          hugs the content (w-max, min-w-full), so the border wraps the header +
          rows tightly with the scrollbar just beyond its right edge.
          scrollbar-gutter:stable keeps that right edge constant whether or not
          the list scrolls, so it always aligns with the toolbar above (which
          reserves the same gutter) and never jumps sideways between pages.
        */}
        <div className='relative min-h-0 flex-1 overflow-auto scrollbar-thin [scrollbar-gutter:stable]'>
          <div className='w-max min-w-full rounded-md border'>
          {isLoading ? (
            skeletonRows
          ) : (
          <div
            aria-busy={isFetching}
            className={cn(
              // Animate only the table when a new search resolves: the old rows
              // stay in place (keepPreviousData) and gently fade while the fetch
              // is in flight, instead of the whole page blanking to skeletons.
              'transition-opacity duration-300 ease-out',
              isFetching && 'pointer-events-none opacity-50'
            )}
          >
          <ShadcnTable>
            <TableHeader>
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow key={headerGroup.id} className='group/row'>
                  {headerGroup.headers.map((header) => {
                    return (
                      <TableHead
                        key={header.id}
                        colSpan={header.colSpan}
                        className={
                          header.column.columnDef.meta?.className ?? ''
                        }
                      >
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                      </TableHead>
                    )
                  })}
                </TableRow>
              ))}
            </TableHeader>
            <TableBody>
              {table.getRowModel().rows?.length ? (
                table.getRowModel().rows.map((row) => (
                  <TableRow
                    key={row.id}
                    data-state={row.getIsSelected() && 'selected'}
                    className={cn(
                      'group/row cursor-pointer transition-colors hover:bg-accent/50'
                    )}
                    onClick={(e) => onRowClick(e, row)}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <TableCell
                        key={cell.id}
                        className={cell.column.columnDef.meta?.className ?? ''}
                        style={{
                          width: cell.column.columnDef.size,
                          minWidth: cell.column.columnDef.minSize,
                          maxWidth: cell.column.columnDef.maxSize,
                        }}
                      >
                        {flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext()
                        )}
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell
                    colSpan={allColumns.length}
                    className='h-24 text-center'
                  >
                    {t('common.table.noResults')}
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </ShadcnTable>
          </div>
          )}
          </div>
        </div>
      </div>
      {bulkActions && totalSelected > 0 && bulkActions}
    </>
  )
}
