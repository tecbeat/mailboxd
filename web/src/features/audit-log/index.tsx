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


import { FixedHeader } from '@/components/layout/fixed-header'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import { DataTablePagination } from '@/components/data-table/data-table-pagination'
import { useAuditLog } from '@/hooks/use-audit-log'
import { useTranslation } from 'react-i18next'
import { getColumns } from './components/columns'
import { AuditLogTable } from './components/table'

export default function AuditLog() {
  const { t } = useTranslation()
  const {
    entries,
    total,
    totalPages,
    page,
    pageSize,
    setPage,
    setPageSize,
    isLoading,
  } = useAuditLog()
  const columns = getColumns(t)

  const handleSetPageSize = (size: number) => {
    setPage(1)
    setPageSize(size)
  }

  return (
    <>
      <FixedHeader />
      <Main fixed>
        <div className="flex h-full min-h-0 w-full flex-col">
          <PageHeader
            className="mb-4 shrink-0"
            title={t('auditLog.title', 'Audit Log')}
            description={t(
              'auditLog.description',
              'Security-relevant events recorded across the server.',
            )}
          />
          <div className="flex min-h-0 w-full flex-1 flex-col">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : entries.length ? (
              <AuditLogTable data={entries} columns={columns} />
            ) : (
              <EmptyState
                title={t('auditLog.empty.title', 'No audit entries')}
                description={t(
                  'auditLog.empty.description',
                  'Security-relevant events will appear here as they happen.',
                )}
              />
            )}
          </div>
          {total > 0 && (
            <div className="shrink-0 pt-2">
              <DataTablePagination
                storageKey="audit-log"
                pageIndex={page - 1}
                pageCount={Math.max(1, Math.ceil(total / pageSize))}
                pageSize={pageSize}
                canPreviousPage={page > 1}
                canNextPage={page < totalPages}
                onFirst={() => setPage(1)}
                onPrevious={() => setPage(page - 1)}
                onNext={() => setPage(page + 1)}
                onLast={() => setPage(totalPages)}
                onPageSizeChange={handleSetPageSize}
              />
            </div>
          )}
        </div>
      </Main>
    </>
  )
}
