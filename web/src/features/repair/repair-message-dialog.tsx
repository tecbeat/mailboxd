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

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { AxiosError } from 'axios'
import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { REPAIR_REPORTS_KEY, repairMessage } from '@/api/repair/api'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/hooks/use-toast'
import { RepairReportView } from './repair-report-view'

interface RepairMessageDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  accountId?: number
  envelopeId?: string
  subject?: string
}

/**
 * Confirms and runs a repair for one message, then shows the report in place.
 * The body remounts on every open, so a reopened dialog never shows a stale
 * report.
 */
export function RepairMessageDialog({ open, onOpenChange, ...target }: RepairMessageDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='sm:max-w-lg'>
        <RepairMessageBody onClose={() => onOpenChange(false)} {...target} />
      </DialogContent>
    </Dialog>
  )
}

function RepairMessageBody({
  onClose,
  accountId,
  envelopeId,
  subject,
}: Omit<RepairMessageDialogProps, 'open' | 'onOpenChange'> & { onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: () => repairMessage(accountId as number, envelopeId as string),
    onSuccess: (report) => {
      if (report.status !== 'complete') {
        queryClient.invalidateQueries({ queryKey: REPAIR_REPORTS_KEY })
      }
    },
    onError: (error: AxiosError) => {
      toast({
        variant: 'destructive',
        title: t('repair_message.requestFailed'),
        description: (error.response?.data as { message?: string })?.message || error.message,
      })
    },
  })

  const report = mutation.data
  const canRepair = accountId != null && !!envelopeId

  return (
    <>
      <DialogHeader>
        <DialogTitle>
          {report ? t('repair_message.reportTitle') : t('repair_message.title')}
        </DialogTitle>
        <DialogDescription>
          {report ? t('repair_message.reportDesc') : t('repair_message.desc')}
        </DialogDescription>
      </DialogHeader>

      {!report && !mutation.isPending && (
        <div className='space-y-3 text-sm'>
          <div className='rounded-md border bg-muted/20 p-3'>
            <div className='text-xs font-medium text-muted-foreground'>{t('mail.subject')}</div>
            <div className='mt-0.5 break-words font-medium'>{subject || '—'}</div>
          </div>
          <p className='text-xs text-muted-foreground'>{t('repair_message.wholeMessageNote')}</p>
        </div>
      )}

      {mutation.isPending && (
        <div role='status' className='flex items-start gap-3 py-2'>
          <LoaderCircle className='mt-0.5 h-4 w-4 shrink-0 animate-spin' />
          <div className='space-y-1'>
            <div className='text-sm font-medium'>{t('repair_message.repairing')}</div>
            <p className='text-xs text-muted-foreground'>{t('repair_message.repairingHint')}</p>
          </div>
        </div>
      )}

      {report && <RepairReportView report={report} />}

      <DialogFooter>
        {report ? (
          <Button onClick={onClose}>{t('common.close')}</Button>
        ) : (
          <>
            <Button variant='outline' onClick={onClose} disabled={mutation.isPending}>
              {t('common.cancel')}
            </Button>
            <Button onClick={() => mutation.mutate()} disabled={!canRepair || mutation.isPending}>
              {t('repair_message.confirm')}
            </Button>
          </>
        )}
      </DialogFooter>
    </>
  )
}
