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

import { CircleCheck, Info, TriangleAlert, type LucideIcon } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { BlobRepairReport, BlobRepairStatus } from '@/api/repair/api'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { cn } from '@/lib/utils'
import { repairStatusLabel, repairStatusVariant } from './status'

const TONE: Record<BlobRepairStatus, string> = {
  complete: 'border-border bg-muted/30',
  repaired: 'border-green-600/40 bg-green-600/10',
  partial: 'border-amber-600/40 bg-amber-600/10',
  failed: 'border-destructive/40 bg-destructive/10',
}

const ICON_TONE: Record<BlobRepairStatus, string> = {
  complete: 'text-muted-foreground',
  repaired: 'text-green-600',
  partial: 'text-amber-600',
  failed: 'text-destructive',
}

/**
 * Renders a repair outcome. `status` is the verdict; the hash lists only name
 * the attachments involved.
 */
export function RepairReportView({
  report,
  className,
}: {
  report: BlobRepairReport
  className?: string
}) {
  const { t } = useTranslation()

  const StatusIcon =
    report.status === 'complete' || report.status === 'repaired' ? CircleCheck : TriangleAlert

  const hint =
    report.status === 'partial'
      ? t('repair_message.partialHint')
      : report.status === 'failed'
        ? t('repair_message.failedHint')
        : null

  return (
    <div className={cn('space-y-3 text-sm', className)}>
      <div className={cn('rounded-md border p-3', TONE[report.status])}>
        <div className='flex items-center gap-2'>
          <StatusIcon className={cn('h-4 w-4 shrink-0', ICON_TONE[report.status])} />
          <span className='font-medium'>{t('repair_message.result')}:</span>
          <Badge variant={repairStatusVariant(report.status)}>
            {repairStatusLabel(t, report.status)}
          </Badge>
        </div>

        {(report.subject || report.message_id) && (
          <div className='mt-2 space-y-0.5 pl-6 text-xs text-muted-foreground'>
            {report.subject && <div className='break-words'>{report.subject}</div>}
            {report.message_id && <div className='break-all font-mono'>{report.message_id}</div>}
          </div>
        )}

        {hint && (
          <div className='mt-2 flex items-start gap-2 pl-6 text-xs text-muted-foreground'>
            <Info className='mt-0.5 h-3.5 w-3.5 shrink-0' />
            <span>{hint}</span>
          </div>
        )}
      </div>

      {report.email_missing_after ? (
        <ReportLine
          icon={TriangleAlert}
          tone='text-amber-600'
          text={t('repair_message.emailMissing')}
        />
      ) : (
        report.email_missing_before && (
          <ReportLine
            icon={CircleCheck}
            tone='text-green-600'
            text={t('repair_message.emailRestored')}
          />
        )
      )}

      <HashList label={t('repair_message.missingBefore')} hashes={report.missing_before} />
      <HashList label={t('repair_message.missingAfter')} hashes={report.missing_after} />

      {report.error && (
        <div className='rounded-md border border-destructive/40 bg-destructive/10 p-3'>
          <div className='text-xs font-medium text-destructive'>{t('common.error')}</div>
          <div className='mt-1 break-all font-mono text-xs text-destructive/90'>{report.error}</div>
        </div>
      )}
    </div>
  )
}

function ReportLine({ icon: Icon, tone, text }: { icon: LucideIcon; tone: string; text: string }) {
  return (
    <div className='flex items-center gap-2'>
      <Icon className={cn('h-4 w-4 shrink-0', tone)} />
      <span>{text}</span>
    </div>
  )
}

function HashList({ label, hashes }: { label: string; hashes: string[] }) {
  if (hashes.length === 0) return null
  return (
    <section aria-label={label}>
      <div className='text-xs font-medium text-muted-foreground'>{label}</div>
      <ScrollArea className='mt-1 max-h-32 rounded border bg-muted/20'>
        <ul className='space-y-0.5 p-2'>
          {hashes.map((hash) => (
            <li key={hash} className='break-all font-mono text-xs'>
              {hash}
            </li>
          ))}
        </ul>
      </ScrollArea>
    </section>
  )
}
