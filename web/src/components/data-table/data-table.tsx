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


import { useEffect, useRef, useState, type ReactNode } from 'react'
import {
  ColumnDef,
  ColumnFiltersState,
  ColumnVisibilityState,
  ReactTable,
  Row,
  RowData,
  SortingState,
  TableOptions,
  flexRender,
  useTable,
} from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { cn } from '@/lib/utils'
import {
  dataTableFeatures,
  getPersistedPageSize,
  type DataTableFeatures,
} from '@/lib/data-table'
import { useTranslation } from 'react-i18next'
import { DataTablePagination } from './data-table-pagination'

interface DataTableProps<TData extends RowData> {
  columns: ColumnDef<DataTableFeatures, TData>[]
  data: TData[]
  // localStorage namespace, e.g. 'accounts' -> mailboxd_accounts_page_size / _sorting.
  storageKey: string
  // Feature-specific toolbar rendered above the table.
  toolbar?: (table: ReactTable<DataTableFeatures, TData>) => ReactNode
  pageSizeOptions?: number[]
  // Persist sorting under mailboxd_<storageKey>_sorting (accounts).
  persistSorting?: boolean
  // Extra class on each body row; receives the row (accounts dims rows being deleted).
  rowClassName?: (row: Row<DataTableFeatures, TData>) => string
  // Class on the bordered wrapper around the table (proxy adds horizontal scroll).
  containerClassName?: string
  // Class on the <Table> element (proxy uses a fixed layout).
  tableClassName?: string
  // Extra class on every body cell (proxy clips overflow).
  cellClassName?: string
  // Render the pagination footer (proxy hides it for a single page of rows).
  showPagination?: boolean
  globalFilterFn?: TableOptions<DataTableFeatures, TData>['globalFilterFn']
}

export function DataTable<TData extends RowData>({
  columns,
  data,
  storageKey,
  toolbar,
  pageSizeOptions,
  persistSorting = false,
  rowClassName,
  containerClassName = 'rounded-md border',
  tableClassName,
  cellClassName,
  showPagination = true,
  globalFilterFn,
}: DataTableProps<TData>) {
  const { t } = useTranslation()
  const sortingStorageKey = `mailboxd_${storageKey}_sorting`
  const [rowSelection, setRowSelection] = useState({})
  const [columnVisibility, setColumnVisibility] = useState<ColumnVisibilityState>({})
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([])
  const [sorting, setSorting] = useState<SortingState>(() => {
    if (!persistSorting) return []
    const saved = localStorage.getItem(sortingStorageKey)
    return saved ? (JSON.parse(saved) as SortingState) : []
  })

  // Persist sorting state to localStorage (opt-in).
  const prevSortingRef = useRef(sorting)
  useEffect(() => {
    if (!persistSorting) return
    if (prevSortingRef.current !== sorting) {
      localStorage.setItem(sortingStorageKey, JSON.stringify(sorting))
      prevSortingRef.current = sorting
    }
  }, [persistSorting, sorting, sortingStorageKey])

  const table = useTable({
    features: dataTableFeatures,
    data,
    columns,
    state: {
      sorting,
      columnVisibility,
      rowSelection,
      columnFilters,
    },
    initialState: {
      pagination: {
        pageIndex: 0,
        pageSize: getPersistedPageSize(storageKey, 10),
      },
    },
    enableRowSelection: true,
    onRowSelectionChange: setRowSelection,
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onColumnVisibilityChange: setColumnVisibility,
    globalFilterFn,
  })

  return (
    <div className='flex h-full min-h-0 flex-col gap-4'>
      {toolbar?.(table)}
      {/*
        The bordered table box is the scroll container: it fills the remaining
        height (flex-1 min-h-0) and scrolls internally, keeping the toolbar
        above and the pagination below it pinned. Degrades to natural flow when
        the parent gives no bounded height. `relative` makes this box the
        containing block for any absolutely-positioned descendants (e.g.
        screen-reader-only labels), so their overflow stays contained here
        instead of inflating the app shell's scroll region.
      */}
      <div className={cn('relative min-h-0 flex-1 overflow-auto scrollbar-thin', containerClassName)}>
        <Table className={tableClassName}>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id} className='group/row'>
                {headerGroup.headers.map((header) => {
                  return (
                    <TableHead
                      key={header.id}
                      colSpan={header.colSpan}
                      className={header.column.columnDef.meta?.className ?? ''}
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
                  className={rowClassName ? rowClassName(row) : 'group/row'}
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell
                      key={cell.id}
                      className={cn(cellClassName, cell.column.columnDef.meta?.className)}
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
                  colSpan={columns.length}
                  className='h-24 text-center'
                >
                  {t('common.table.noResults')}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
      {showPagination && (
        <div className='shrink-0'>
          <DataTablePagination
            table={table}
            storageKey={storageKey}
            pageSizeOptions={pageSizeOptions}
          />
        </div>
      )}
    </div>
  )
}
