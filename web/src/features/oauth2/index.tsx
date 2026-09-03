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
import { Button } from '@/components/ui/button'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { ActionDialog } from './components/action-dialog'
import { getColumns } from './components/columns'
import { TokenDeleteDialog } from './components/delete-dialog'
import { Oauth2Table } from './components/oauth2-table'
import OAuth2Provider, {
  type OAuth2DialogType,
} from './context'
import { Plus } from 'lucide-react'
import { OAuth2Entity } from './data/schema'
import { useQuery } from '@tanstack/react-query'
import { get_oauth2_list } from '@/api/oauth2/api'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import { AuthorizeDialog } from './components/authorize-dialog'
import { FixedHeader } from '@/components/layout/fixed-header'
import { useTranslation } from 'react-i18next'
import { useCurrentUser } from '@/hooks/use-current-user'

export default function OAuth2() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<OAuth2Entity | null>(null)
  const [open, setOpen] = useDialogState<OAuth2DialogType>(null)
  const { require_any_permission } = useCurrentUser()

  const { data: oauth2List, isLoading } = useQuery({
    queryKey: ['oauth2-list'],
    queryFn: get_oauth2_list,
  })

  const columns = getColumns(t)

  return (
    <OAuth2Provider value={{ open, setOpen, currentRow, setCurrentRow }}>
      <FixedHeader />
      <Main fixed>
        <div className="flex h-full min-h-0 flex-col">
          <PageHeader
            className="mb-2 shrink-0"
            title={t('oauth2.title')}
            description={t('oauth2.description')}
            actions={
              <Button className="space-x-1" disabled={!require_any_permission(['system:root'])} onClick={() => setOpen("add")}>
                <span>{t('common.add')}</span>
                <Plus size={18} />
              </Button>
            }
          />
          <div className="flex min-h-0 w-full flex-1 flex-col py-1">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : oauth2List?.items.length ? (
              <Oauth2Table data={oauth2List.items} columns={columns} />
            ) : (
              <EmptyState
                title={t('oauth2.noConfigurations')}
                description={t('oauth2.noConfigurationsDesc')}
              />
            )}
          </div>
        </div>
      </Main>
      <ActionDialog
        key='oauth2-add'
        open={open === 'add'}
        onOpenChange={() => setOpen('add')}
      />

      {currentRow && (
        <>
          <ActionDialog
            key={`oauth2-edit-${currentRow.id}`}
            open={open === 'edit'}
            onOpenChange={() => {
              setOpen('edit')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          <TokenDeleteDialog
            key={`oauth2-delete-${currentRow.id}`}
            open={open === 'delete'}
            onOpenChange={() => {
              setOpen('delete')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />

          <AuthorizeDialog
            key={`oauth2-authorize-${currentRow.id}`}
            open={open === 'authorize'}
            onOpenChange={() => {
              setOpen('authorize')
              setTimeout(() => {
                setCurrentRow(null)
              }, 500)
            }}
            currentRow={currentRow}
          />
        </>
      )}
    </OAuth2Provider>
  )
}
