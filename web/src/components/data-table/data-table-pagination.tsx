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
  ChevronLeft as ChevronLeftIcon,
  ChevronRight as ChevronRightIcon,
  ChevronsLeft as DoubleArrowLeftIcon,
  ChevronsRight as DoubleArrowRightIcon,
} from 'lucide-react'
import { ReactTable, RowData } from '@tanstack/react-table'
import {
  setPersistedPageSize,
  type DataTableFeatures,
} from '@/lib/data-table'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'

const DEFAULT_PAGE_SIZE_OPTIONS = [10, 20, 30, 40, 50, 100, 150]

// localStorage namespace; the chosen size persists as mailboxd_<storageKey>_page_size.
interface TablePaginationProps<TData extends RowData> {
  table: ReactTable<DataTableFeatures, TData>
  storageKey: string
  pageSizeOptions?: number[]
  showSelected?: boolean
  showPageSizeSelector?: boolean
}

interface ControlledPaginationProps {
  storageKey: string
  pageIndex: number
  pageCount: number
  pageSize: number
  canPreviousPage: boolean
  canNextPage: boolean
  onFirst: () => void
  onPrevious: () => void
  onNext: () => void
  onLast: () => void
  onPageSizeChange: (size: number) => void
  pageSizeOptions?: number[]
  showPageSizeSelector?: boolean
}

type DataTablePaginationProps<TData extends RowData> =
  | TablePaginationProps<TData>
  | ControlledPaginationProps

interface PaginationViewModel {
  pageIndex: number
  pageCount: number
  pageSize: number
  canPrev: boolean
  canNext: boolean
  goFirst: () => void
  goPrev: () => void
  goNext: () => void
  goLast: () => void
  setSize: (size: number) => void
}

function isTableMode<TData extends RowData>(
  props: DataTablePaginationProps<TData>,
): props is TablePaginationProps<TData> {
  return 'table' in props
}

export function DataTablePagination<TData extends RowData>(
  props: DataTablePaginationProps<TData>,
) {
  const { t } = useTranslation()
  const {
    storageKey,
    pageSizeOptions = DEFAULT_PAGE_SIZE_OPTIONS,
    showPageSizeSelector = true,
  } = props

  const tableMode = isTableMode(props)
  const showSelected = tableMode ? (props.showSelected ?? false) : false

  const view: PaginationViewModel = tableMode
    ? {
        pageIndex: props.table.state.pagination.pageIndex,
        pageCount: props.table.getPageCount(),
        pageSize: props.table.state.pagination.pageSize,
        canPrev: props.table.getCanPreviousPage(),
        canNext: props.table.getCanNextPage(),
        goFirst: () => props.table.setPageIndex(0),
        goPrev: () => props.table.previousPage(),
        goNext: () => props.table.nextPage(),
        goLast: () => props.table.setPageIndex(props.table.getPageCount() - 1),
        setSize: (size) => {
          setPersistedPageSize(storageKey, size)
          props.table.setPageSize(size)
        },
      }
    : {
        pageIndex: props.pageIndex,
        pageCount: props.pageCount,
        pageSize: props.pageSize,
        canPrev: props.canPreviousPage,
        canNext: props.canNextPage,
        goFirst: props.onFirst,
        goPrev: props.onPrevious,
        goNext: props.onNext,
        goLast: props.onLast,
        setSize: (size) => {
          setPersistedPageSize(storageKey, size)
          props.onPageSizeChange(size)
        },
      }

  return (
    <div className='flex items-center justify-between overflow-auto px-2'>
      {showSelected && tableMode && (
        <div className='hidden flex-1 text-sm text-muted-foreground sm:block'>
          {t('table.pagination.selected', {
            selected: props.table.getFilteredSelectedRowModel().rows.length,
            total: props.table.getFilteredRowModel().rows.length,
          })}
        </div>
      )}
      {!showPageSizeSelector && (
        <div className='hidden flex-1 text-sm text-muted-foreground sm:block'>
          {t('table.pagination.fixed_page_size', { size: 10 })}
        </div>
      )}
      <div className='flex items-center sm:space-x-6 lg:space-x-8 ml-auto'>
        {showPageSizeSelector && (
          <div className='flex items-center space-x-2'>
            <p className='hidden text-sm font-medium sm:block'>
              {t('table.pagination.rows_per_page')}
            </p>
            <Select
              value={`${view.pageSize}`}
              onValueChange={(value) => view.setSize(Number(value))}
            >
              <SelectTrigger className='h-8 w-[70px]'>
                <SelectValue placeholder={view.pageSize} />
              </SelectTrigger>
              <SelectContent side='top'>
                {pageSizeOptions.map((pageSize) => (
                  <SelectItem key={pageSize} value={`${pageSize}`}>
                    {pageSize}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}
        <div className='flex w-[130px] items-center justify-center text-sm font-medium'>
          {t('table.pagination.page_info', {
            page: view.pageIndex + 1,
            total: view.pageCount,
          })}
        </div>
        <div className='flex items-center space-x-2'>
          <Button
            variant='outline'
            className='hidden h-8 w-8 p-0 lg:flex'
            onClick={view.goFirst}
            disabled={!view.canPrev}
          >
            <span className='sr-only'>{t('table.pagination.first')}</span>
            <DoubleArrowLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={view.goPrev}
            disabled={!view.canPrev}
          >
            <span className='sr-only'>{t('table.pagination.previous')}</span>
            <ChevronLeftIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='h-8 w-8 p-0'
            onClick={view.goNext}
            disabled={!view.canNext}
          >
            <span className='sr-only'>{t('table.pagination.next')}</span>
            <ChevronRightIcon className='h-4 w-4' />
          </Button>
          <Button
            variant='outline'
            className='hidden h-8 w-8 p-0 lg:flex'
            onClick={view.goLast}
            disabled={!view.canNext}
          >
            <span className='sr-only'>{t('table.pagination.last')}</span>
            <DoubleArrowRightIcon className='h-4 w-4' />
          </Button>
        </div>
      </div>
    </div>
  )
}
