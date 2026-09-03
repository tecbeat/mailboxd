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

import { useState } from 'react'
import useDialogState from '@/hooks/use-dialog-state'
import { PageHeader } from '@/components/layout/page-header'
import { getColumns } from './components/columns'
import { ApiTokenDeleteDialog } from './components/delete-dialog'
import { ApiTokensTable } from './components/table'
import ApiTokenProvider, {
  type ApiTokenDialogType,
} from './context'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import Logo from '@/assets/logo.svg'
import { AccessToken, list_access_tokens } from '@/api/users/api'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

export default function ApiTokens() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<AccessToken | null>(null)
  const [open, setOpen] = useDialogState<ApiTokenDialogType>(null)

  const { data: apiTokens, isLoading } = useQuery({
    queryKey: ['access-token-list'],
    queryFn: list_access_tokens,
  })

  const columns = getColumns(t)

  return (
    <div className="w-full">
      <ApiTokenProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
        <div className="w-full">
          <PageHeader
            className="mb-4"
            title={t('users.api_tokens.title', 'API Tokens')}
            description={t('users.api_tokens.description', 'System-wide API tokens for programmatic access.')}
          />
          {isLoading ? (
            <TableSkeleton columns={columns.length} rows={10} />
          ) : apiTokens?.length ? (
            <div className="overflow-x-auto">
              <ApiTokensTable data={apiTokens} columns={columns} />
            </div>
          ) : (
            <EmptyState
              icon={
                <img
                  src={Logo}
                  className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
                  alt="mailboxd icon"
                />
              }
              title={t('users.api_tokens.empty.title')}
              description={t('users.api_tokens.empty.description')}
            />
          )}

          {currentRow && (
            <ApiTokenDeleteDialog
              key={`api-token-delete-${currentRow.token}`}
              currentRow={currentRow}
              open={open === 'delete'}
              onOpenChange={() => {
                setOpen(null)
                setTimeout(() => setCurrentRow(null), 500)
              }}
            />
          )}
        </div>
      </ApiTokenProvider>
    </div>
  )
}
