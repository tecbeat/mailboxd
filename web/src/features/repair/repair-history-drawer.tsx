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

import { Fragment, useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronDown, LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { listRepairReports, REPAIR_REPORTS_KEY } from '@/api/repair/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { DateCell } from '@/features/mail-list/table/date-cell'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { cn } from '@/lib/utils'
import { RepairReportView } from './repair-report-view'
import { repairStatusLabel, repairStatusVariant } from './status'

/** The server's maximum; it also drops records the caller may not repair. */
const HISTORY_LIMIT = 200

const COLUMNS = 6

interface RepairHistoryDrawerProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/** The most recent repair attempts, newest first. Read-only. */
export function RepairHistoryDrawer({ open, onOpenChange }: RepairHistoryDrawerProps) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})
  const { getEmailById } = useMinimalAccountList()

  const { data: records = [], isLoading, error } = useQuery({
    queryKey: [...REPAIR_REPORTS_KEY, HISTORY_LIMIT],
    queryFn: () => listRepairReports(HISTORY_LIMIT),
    enabled: open,
  })

  const toggle = (id: string) => setExpanded((prev) => ({ ...prev, [id]: !prev[id] }))

  const placeholder = (content: ReactNode, className?: string) => (
    <TableRow>
      <TableCell colSpan={COLUMNS} className={cn('py-8 text-center', className)}>
        {content}
      </TableCell>
    </TableRow>
  )

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side='right' className='w-full gap-0 sm:max-w-4xl'>
        <SheetHeader className='pb-2'>
          <SheetTitle>{t('repair_message.history')}</SheetTitle>
          <SheetDescription>
            {t('repair_message.historyHint', { limit: HISTORY_LIMIT })}
          </SheetDescription>
        </SheetHeader>

        <div className='flex-1 px-4 pb-6'>
          <div className='overflow-x-auto rounded-md border'>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className='w-10' />
                  <TableHead className='text-xs'>{t('auditLog.columns.timestamp')}</TableHead>
                  <TableHead className='text-xs'>{t('mail.subject')}</TableHead>
                  <TableHead className='text-xs'>{t('repair_message.operator')}</TableHead>
                  <TableHead className='text-xs'>{t('mail.account')}</TableHead>
                  <TableHead className='text-xs'>{t('repair_message.result')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {isLoading
                  ? placeholder(
                      <LoaderCircle
                        role='status'
                        aria-label={t('common.loading')}
                        className='mx-auto h-4 w-4 animate-spin text-muted-foreground'
                      />
                    )
                  : error
                    ? placeholder(t('repair_message.historyFailed'), 'text-destructive')
                    : records.length === 0
                      ? placeholder(t('repair_message.historyEmpty'), 'text-muted-foreground')
                      : records.map((record) => {
                          const isOpen = !!expanded[record.id]
                          return (
                            <Fragment key={record.id}>
                              <TableRow>
                                <TableCell className='w-10'>
                                  <Button
                                    variant='ghost'
                                    size='icon'
                                    className='h-7 w-7'
                                    aria-expanded={isOpen}
                                    aria-label={
                                      isOpen
                                        ? t('repair_message.hideDetails')
                                        : t('repair_message.showDetails')
                                    }
                                    onClick={() => toggle(record.id)}
                                  >
                                    <ChevronDown
                                      className={cn('h-4 w-4 transition-transform', isOpen && 'rotate-180')}
                                    />
                                  </Button>
                                </TableCell>
                                <TableCell>
                                  <DateCell date={record.created_at} />
                                </TableCell>
                                <TableCell className='max-w-md truncate text-xs'>
                                  {record.subject || '—'}
                                </TableCell>
                                <TableCell className='text-xs'>{record.operator}</TableCell>
                                <TableCell className='text-xs text-muted-foreground'>
                                  {getEmailById(record.account_id) ?? record.account_id}
                                </TableCell>
                                <TableCell>
                                  <Badge variant={repairStatusVariant(record.report.status)}>
                                    {repairStatusLabel(t, record.report.status)}
                                  </Badge>
                                </TableCell>
                              </TableRow>
                              {isOpen && (
                                <TableRow className='bg-muted/40'>
                                  <TableCell colSpan={COLUMNS} className='p-4'>
                                    <RepairReportView report={record.report} />
                                  </TableCell>
                                </TableRow>
                              )}
                            </Fragment>
                          )
                        })}
              </TableBody>
            </Table>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}
