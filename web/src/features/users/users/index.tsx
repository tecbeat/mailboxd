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
import { PageHeader } from '@/components/layout/page-header'
import { UserActionDialog } from './components/action-dialog'
import { getColumns } from './components/columns'
import { UserDeleteDialog } from './components/delete-dialog'
import { UsersTable } from './components/table'
import UserProvider, {
  type UserDialogType,
} from './context'
import { Plus } from 'lucide-react'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import { list_users, User } from '@/api/users/api'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { useRoles } from '@/hooks/use-roles'
import { UserApiTokensDialog } from './components/api-tokens-dialog'

export default function Users() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<User | null>(null)
  const [open, setOpen] = useDialogState<UserDialogType>(null)
  const { global } = useRoles()

  const { data: users, isLoading } = useQuery({
    queryKey: ['user-list'],
    queryFn: list_users,
  })
  const columns = getColumns(t, global.roles!)

  return (
    <div className="flex h-full min-h-0 w-full flex-col">
      <UserProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
        <div className="flex min-h-0 w-full flex-1 flex-col">
          <PageHeader
            className="mb-4 shrink-0"
            title={t('users.title', 'Users')}
            description={t('users.description', 'Manage system users and their roles.')}
            actions={
              <Button onClick={() => setOpen('add')}>
                <Plus className="mr-2 h-4 w-4" /><span>{t('users.buttons.add')}</span>
              </Button>
            }
          />

          <div className="flex min-h-0 w-full flex-1 flex-col">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : users?.length ? (
              <UsersTable data={users} columns={columns} />
            ) : (
              <EmptyState
                title={t('users.empty.title')}
                description={t('users.empty.description')}
              />
            )}
          </div>

          <UserActionDialog
            key="user-add"
            open={open === 'add'}
            onOpenChange={() => setOpen(null)}
          />

          {currentRow && (
            <>
              <UserActionDialog
                key={`user-edit-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'edit'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />

              <UserApiTokensDialog
                key={`api-tokens-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'api-tokens'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />

              <UserDeleteDialog
                key={`user-delete-${currentRow.id}`}
                currentRow={currentRow}
                open={open === 'delete'}
                onOpenChange={() => {
                  setOpen(null)
                  setTimeout(() => setCurrentRow(null), 500)
                }}
              />
            </>
          )}
        </div>
      </UserProvider>
    </div>
  )
}