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
import { ProxyActionDialog } from './components/action-dialog'
import { getColumns } from './components/columns'
import { ProxyDeleteDialog } from './components/delete-dialog'
import { ProxyTable } from './components/table'
import ProxyProvider, {
  type ProxyDialogType,
} from './context'
import { Plus } from 'lucide-react'
import { TableSkeleton } from '@/components/table-skeleton'
import { EmptyState } from '@/components/ui/empty-state'
import useProxyList from '@/hooks/use-proxy'
import { useTranslation } from 'react-i18next'
import { Proxy } from '@/api/system/api'
import { useCurrentUser } from '@/hooks/use-current-user'


export default function ProxyManagerPage() {
  const { t } = useTranslation()
  const [currentRow, setCurrentRow] = useState<Proxy | null>(null)
  const [open, setOpen] = useDialogState<ProxyDialogType>(null)
  const { require_any_permission } = useCurrentUser()
  const { proxyList, isLoading } = useProxyList()
  const columns = getColumns(t)

  return (
    <div className="flex h-full min-h-0 w-full flex-col">
      <ProxyProvider value={{ open, setOpen, currentRow, setCurrentRow }}>
        <div className="flex min-h-0 w-full flex-1 flex-col">
          <PageHeader
            className="mb-4 shrink-0"
            title={t('settings.proxyTitle', 'Network Proxy')}
            description={t('settings.proxyDescription', 'Configure proxy servers used for account connections.')}
            actions={
              <Button disabled={!require_any_permission(['system:root'])} onClick={() => setOpen('add')}>
                <Plus className="mr-2 h-4 w-4" /><span>{t('settings.add')}</span>
              </Button>
            }
          />
          <div className="flex min-h-0 w-full flex-1 flex-col py-1">
            {isLoading ? (
              <TableSkeleton columns={columns.length} rows={10} />
            ) : proxyList?.length ? (
              <ProxyTable data={proxyList} columns={columns} />
            ) : (
              <EmptyState
                title={t('settings.noProxies')}
                description={t('settings.noProxiesDesc')}
              />
            )}
          </div>
        </div>
        <ProxyActionDialog
          key="Proxy-add"
          open={open === 'add'}
          onOpenChange={() => setOpen(null)}
        />

        {currentRow && (
          <>
            <ProxyActionDialog
              key={`Proxy-edit-${currentRow.id}`}
              open={open === 'edit'}
              currentRow={currentRow}
              onOpenChange={() => {
                setOpen(null)
                setTimeout(() => setCurrentRow(null), 500)
              }}
            />

            <ProxyDeleteDialog
              key={`Proxy-delete-${currentRow.id}`}
              open={open === 'delete'}
              currentRow={currentRow}
              onOpenChange={() => {
                setOpen(null)
                setTimeout(() => setCurrentRow(null), 500)
              }}
            />
          </>
        )}
      </ProxyProvider>
    </div>
  )
}
