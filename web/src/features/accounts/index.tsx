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
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { useColumns } from './components/columns'
import { AccountDeleteDialog } from './components/delete-dialog'
import { AccountTable } from './components/table'
import AccountProvider from './context/provider'
import { type AccountDialogType } from './context'
import { Mail, Database } from 'lucide-react'
import { AccountDetailDrawer } from './components/account-detail'
import { AccountModel, list_accounts } from '@/api/account/api'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import { useQuery } from '@tanstack/react-query'
import { OAuth2TokensDialog } from './components/oauth2-tokens'
import { RunningStateDialog } from './components/running-state-dialog'
import { FixedHeader } from '@/components/layout/fixed-header'
import { DownloadFoldersDialog } from './components/download-folders'
import { NoSyncAccountDialog } from './components/nosync-dialog'
import { useTranslation } from 'react-i18next'
import { AccountAccessAssignmentDialog } from './components/access-assignment-dialog'
import { useCurrentUser } from '@/hooks/use-current-user'

export default function Accounts() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const columns = useColumns()
  // Dialog states
  const [currentRow, setCurrentRow] = useState<AccountModel | null>(null)
  const [open, setOpen] = useDialogState<AccountDialogType>(null)
  const { require_any_permission } = useCurrentUser()

  const { data: accountList, isLoading } = useQuery({
    queryKey: ['account-list'],
    queryFn: list_accounts,
    refetchInterval: (query) => {
      const items = (query.state.data as { items?: { deleting?: boolean }[] })?.items;
      return items?.some((item) => item.deleting) ? 5000 : false;
    },
  })

  const hasAccounts = accountList != null && accountList.items.length > 0;

  return (
    <AccountProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
      <FixedHeader />

      <Main fixed>
        <div className="flex h-full min-h-0 flex-col">
          <PageHeader
            className='mb-4 shrink-0'
            title={t('accounts.title')}
            description={t('accounts.description')}
            actions={require_any_permission(['system:root', 'account:create']) && (
              <div className="flex gap-2">
                <Button onClick={() => navigate({ to: '/accounts/new' })}>
                  <Mail className="mr-1.5 h-4 w-4" />
                  {t('accounts.imapAccount')}
                </Button>
                <Button variant="outline" onClick={() => setOpen("add-nosync")}>
                  <Database className="mr-1.5 h-4 w-4" />
                  {t('accounts.noSyncAccount')}
                </Button>
              </div>
            )}
          />

          <div className='flex min-h-0 w-full flex-1 flex-col py-1'>
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : hasAccounts ? (
              <AccountTable data={accountList.items} columns={columns} />
            ) : (
              <EmptyState
                title={t('accounts.noAccountConfigurations')}
                description={t('accounts.noAccountConfigurationsDesc')}
              />
            )}
          </div>
        </div>
      </Main>

      <NoSyncAccountDialog
        key='nosync-account-add'
        open={open === 'add-nosync'}
        onOpenChange={() => setOpen('add-nosync')}
      />

      {currentRow && (
        <>
          <NoSyncAccountDialog
            key={`nosync-account-edit-${currentRow.id}`}
            open={open === 'edit-nosync'}
            onOpenChange={() => {
              setOpen('edit-nosync')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          {require_any_permission(['system:root', 'account:read_details'], currentRow.id) && <RunningStateDialog
            key='running-state'
            open={open === 'running-state'}
            onOpenChange={() => {
              setOpen('running-state')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />}

          <AccountDeleteDialog
            key={`account-delete-${currentRow.id}`}
            open={open === 'delete'}
            onOpenChange={() => {
              setOpen('delete')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          <DownloadFoldersDialog
            key={`sync-folders-${currentRow.id}`}
            open={open === 'sync-folders'}
            onOpenChange={() => {
              setOpen('sync-folders')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
          {require_any_permission(['system:root', 'account:manage'], currentRow.id) && <AccountAccessAssignmentDialog
            key={`access-assign-${currentRow.id}`}
            open={open === 'access-assign'}
            onOpenChange={() => {
              setOpen('access-assign')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />}

          <AccountDetailDrawer
            open={open === 'detail'}
            onOpenChange={() => setOpen('detail')}
            currentRow={currentRow}
          />
          {require_any_permission(['system:root', 'account:manage'], currentRow.id) && <OAuth2TokensDialog open={open === 'oauth2'}
            onOpenChange={() => setOpen('oauth2')}
            currentRow={currentRow}
          />}
        </>
      )}
    </AccountProvider>
  )
}
