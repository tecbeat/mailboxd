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
import { useTranslation } from 'react-i18next'
import {
  dataTableFeatures,
  getPersistedPageSize,
  type DataTableFeatures,
} from '@/lib/data-table'
import { cn } from '@/lib/utils'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
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
  const [columnVisibility, setColumnVisibility] =
    useState<ColumnVisibilityState>({})
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
    <div className='flex min-h-0 flex-col gap-4'>
      {toolbar?.(table)}
      {/*
        Hug-content layout: this root is a normal flex child (flex: 0 1 auto),
        so it is only as tall as toolbar + rows + pagination and does NOT
        stretch to fill the parent — no empty bordered box under a short list.

        Both scroll axes live on the OUTER layer so BOTH scrollbars sit OUTSIDE
        the bordered box — the horizontal bar in its own strip below it, the
        vertical bar in its own strip to its right — never on top of the rows:

        OUTER (this div) owns BOTH scroll axes (overflow-auto) and fills the
        remaining height (flex-1 min-h-0). When the table is wider than the
        column its horizontal scrollbar sits below the bordered box; when the
        rows overflow its height the vertical scrollbar runs down the right,
        just outside the border. scrollbar-thin styles both.

        MIDDLE owns the border (containerClassName) and hugs the table content
        in BOTH dimensions — only as wide as the table (w-max, min-w-full) and
        only as tall as its rows (no height cap, no scroll of its own). So the
        border wraps the header + rows tightly and the OUTER's scrollbars stay
        outside it. The sticky header (th: sticky top-0) sticks to the OUTER
        viewport, so it stays visible while the border scrolls with the content
        — the vertical counterpart of the left/right border sliding under a
        horizontal scroll. `relative` makes it the containing block for any
        absolutely-positioned descendants (e.g. screen-reader-only labels).

        FOOTGUN: `w-max` is max-content. A `table-fixed` + `w-full` table inside
        a max-content wrapper degenerates to a ~1,000,000px width. Such callers
        must override the width by passing `w-full` in containerClassName (see
        the proxy table), which twMerge resolves ahead of the default `w-max`.
      */}
      <div className='min-h-0 flex-1 overflow-auto scrollbar-thin'>
        <div
          className={cn(
            'relative w-max min-w-full',
            containerClassName
          )}
        >
          <Table className={tableClassName}>
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
                    className={rowClassName ? rowClassName(row) : 'group/row'}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <TableCell
                        key={cell.id}
                        className={cn(
                          cellClassName,
                          cell.column.columnDef.meta?.className
                        )}
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
