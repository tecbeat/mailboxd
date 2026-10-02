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


import { Row } from '@tanstack/react-table'
import { type DataTableFeatures } from '@/lib/data-table'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useAccountContext } from '../context'
import { Mailbox, MessageSquareMore, Settings, MoreHorizontal, Pencil, Play, Square, ShieldCheck, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useCurrentUser } from '@/hooks/use-current-user'
import { AccountModel, cancel_account_download, start_account_download } from '@/api/account/api'
import { toast } from '@/hooks/use-toast'
import { useNavigate } from '@tanstack/react-router'
import { AxiosError } from 'axios'

interface DataTableRowActionsProps {
  row: Row<DataTableFeatures, AccountModel>
}

export function DataTableRowActions({ row }: DataTableRowActionsProps) {
  const { t } = useTranslation()
  const { setOpen, setCurrentRow } = useAccountContext()
  const navigate = useNavigate()

  const account_type = row.original.account_type;
  const { require_any_permission } = useCurrentUser()

  const hasPermission = require_any_permission(['system:root', 'account:manage'], row.original.id);
  const hasReadPermission = require_any_permission(['system:root', 'account:read_details'], row.original.id);

  const isDeleting = row.original.deleting === true;

  const canShowAnyAction =
    !isDeleting && (
    (hasPermission) ||
    (account_type === 'IMAP' && hasPermission) ||
    (account_type === 'IMAP' && hasReadPermission)
  );

  const showDownload = !isDeleting && account_type === 'IMAP' && hasPermission;

  const handleStartDownload = async () => {
    try {
      await start_account_download(row.original.id);
      toast({ title: t('accounts.downloadStarted') });
    } catch (error: unknown) {
      const axiosError = error as AxiosError<{ message?: string }>
      toast({
        variant: "destructive",
        title: t('accounts.downloadFailed'),
        description: axiosError.response?.data?.message || axiosError.message
      });
    }
  }


  const handleCancelDownload = async () => {
    try {
      await cancel_account_download(row.original.id);
      toast({ title: t('accounts.downloadCancelled') });
    } catch (error: unknown) {
      const axiosError = error as AxiosError<{ message?: string }>
      toast({
        variant: "destructive",
        title: t('accounts.cancelFailed'),
        description: axiosError.response?.data?.message || axiosError.message
      });
    }
  }

  return (
    <>
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild disabled={!canShowAnyAction}>
          <Button
            variant='ghost'
            className='flex h-8 w-8 p-0 data-[state=open]:bg-muted'
          >
            <MoreHorizontal className='h-4 w-4' />
            <span className='sr-only'>{t('accounts.openMenu')}</span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align='end' className='w-[220px]'>
          {hasPermission && <DropdownMenuItem
            onClick={() => {
              if (account_type === "IMAP" || account_type === "JMAP") {
                navigate({ to: '/accounts/$id/settings', params: { id: String(row.original.id) } });
              } else {
                setCurrentRow(row.original)
                setOpen("edit-nosync");
              }
            }}
          >
            {t('accounts.edit')}
            <DropdownMenuShortcut>
              <Pencil />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {(account_type === "IMAP" || account_type === "JMAP") && hasPermission && <DropdownMenuItem
            onClick={() => {
              navigate({ to: '/accounts/$id/settings', params: { id: String(row.original.id) } });
            }}
          >
            {t('accounts.settings.settings')}
            <DropdownMenuShortcut>
              <Settings />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {account_type === "IMAP" && hasPermission && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('sync-folders')
            }}
          >
            {t('accounts.selectMailboxes')}
            <DropdownMenuShortcut>
              <Mailbox />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {account_type === "IMAP" && hasReadPermission && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('detail')
            }}
          >
            {t('accounts.detail')}
            <DropdownMenuShortcut>
              <MessageSquareMore />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {hasPermission && <DropdownMenuSeparator />}
          {hasPermission && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('access-assign')
            }}
          >
            <span>{t('accounts.accessControl')}</span>
            <DropdownMenuShortcut>
              <ShieldCheck />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
          {hasPermission && <DropdownMenuSeparator />}

          {showDownload && (
            <DropdownMenuItem onClick={handleStartDownload}>
              {t('accounts.startDownload')}
              <DropdownMenuShortcut>
                <Play />
              </DropdownMenuShortcut>
            </DropdownMenuItem>
          )}


          {showDownload && (
            <DropdownMenuItem onClick={handleCancelDownload}>
              {t('accounts.cancelDownload')}
              <DropdownMenuShortcut>
                <Square />
              </DropdownMenuShortcut>
            </DropdownMenuItem>
          )}
          {showDownload && <DropdownMenuSeparator />}

          {hasPermission && <DropdownMenuItem
            onClick={() => {
              setCurrentRow(row.original)
              setOpen('delete')
            }}
            className='text-destructive focus:text-destructive'
          >
            {t('accounts.delete')}
            <DropdownMenuShortcut>
              <Trash2 />
            </DropdownMenuShortcut>
          </DropdownMenuItem>}
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  )
}
