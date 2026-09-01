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


import { dateFnsLocaleMap, formatBytes } from "@/lib/utils"
import { format, formatDistanceToNow } from "date-fns"
import { MessageSquareText, Paperclip } from "lucide-react"
import { EmailEnvelope } from "@/api"
import { MailBulkActions } from "./bulk-actions"
import { useTranslation } from 'react-i18next'
import { enUS } from "date-fns/locale"
import { ColumnDef } from "@tanstack/react-table"
import { type MailTableFeatures } from "@/lib/data-table"
import LongText from "@/components/long-text"
import { DataTableColumnHeader } from "@/features/mail-list/table/data-table-column-header"
import { MailListDataTable } from "@/features/mail-list/mail-list-data-table"
import { DataTableRowActions } from "./table/data-table-row-actions"
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { DataTableToolbar } from "./table/toolbar"
import { HoverCard, HoverCardContent, HoverCardTrigger } from "@/components/ui/hover-card"
import { useSearchMessages } from "@/hooks/use-search-messages"

interface MailListProps {
  items: EmailEnvelope[]
  isLoading: boolean
  onEnvelopeChanged: (envelope: EmailEnvelope) => void
  setSortBy: (sortBy: "DATE" | "SIZE") => void
  setSortOrder: (value: "desc" | "asc") => void
}

export function MailListTable({
  items,
  isLoading,
  onEnvelopeChanged,
  setSortBy,
  setSortOrder
}: MailListProps) {
  const { t, i18n } = useTranslation()

  const locale = dateFnsLocaleMap[i18n.language.toLowerCase()] ?? enUS

  const columns: ColumnDef<MailTableFeatures, EmailEnvelope>[] = [
    {
      accessorKey: "source",
      header: t('search.source'),
      cell: ({ row }) => {
        const { from, account_email, account_name, mailbox_name, account_id, mailbox_id } = row.original;
        const { setFilter } = useSearchMessages();
        const accountPrefix = account_name ?? account_email.split('@')[0];

        return (
          <div className="flex flex-col py-1.5 min-w-0 group">
            <div
              className="cursor-pointer hover:text-primary transition-colors flex items-center gap-1.5"
              onClick={(e) => {
                e.stopPropagation();
                setFilter((prev: any) => ({ ...prev, from: from }));
              }}
            >
              <LongText className="text-xs truncate">
                {from}
              </LongText>
            </div>

            <div className="flex items-center gap-1 mt-1 text-[10px] text-muted-foreground/70">
              <span
                className="truncate max-w-[90px] hover:text-primary cursor-pointer transition-colors"
                title={account_email}
                onClick={(e) => {
                  e.stopPropagation();
                  setFilter((prev: any) => ({ ...prev, account_ids: [account_id], mailbox_ids: undefined }));
                }}
              >
                {accountPrefix}
              </span>

              <span className="shrink-0 opacity-40">/</span>
              <span
                className="truncate max-w-[70px] hover:text-primary cursor-pointer transition-colors"
                onClick={(e) => {
                  e.stopPropagation();
                  setFilter((prev: any) => ({ ...prev, account_ids: [account_id], mailbox_ids: [mailbox_id] }));
                }}
              >
                {mailbox_name}
              </span>
            </div>
          </div>
        );
      },
      meta: { className: 'text-left text-xs' },
    },
    {
      accessorKey: "to",
      header: t('search.to'),
      cell: ({ row }) => {
        const recipients: string[] = row.original.to || [];
        const { setFilter } = useSearchMessages();
        const MAX_VISIBLE = 2;
        const visible = recipients.slice(0, MAX_VISIBLE);
        const hidden = recipients.slice(MAX_VISIBLE);

        const handleClick = (e: React.MouseEvent, email: string) => {
          e.stopPropagation();
          setFilter((prev: Record<string, any>) => ({ ...prev, to: email }));
        };

        return (
          <div className="group relative flex items-center w-full min-w-0 px-2 py-1.5 overflow-hidden">
            <div className="absolute left-0 top-0 bottom-0 w-[2px] bg-primary opacity-0 group-hover:opacity-100 transition-opacity" />
            <div className="text-xs flex flex-wrap items-center gap-x-1 gap-y-0.5 min-w-0 flex-1">
              {visible.map((email, index) => (
                <span key={index} className="flex items-center">
                  <button
                    type="button"
                    onClick={(e) => handleClick(e, email)}
                    className="hover:text-primary hover:underline transition-colors truncate max-w-[160px]"
                  >
                    {email}
                  </button>
                  {index < visible.length - 1 && (
                    <span className="text-muted-foreground ml-0.5">,</span>
                  )}
                </span>
              ))}

              {hidden.length > 0 && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span
                      onClick={(e) => e.stopPropagation()}
                      className="inline-flex items-center justify-center rounded-full bg-muted text-muted-foreground px-1.5 py-0.5 text-[10px] leading-none cursor-default hover:bg-accent hover:text-accent-foreground transition-colors shrink-0"
                    >
                      +{hidden.length}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent className="max-w-[480px] bg-popover text-popover-foreground border">
                    <div className="flex flex-col gap-1">
                      {hidden.map((email, idx) => (
                        <button
                          key={idx}
                          type="button"
                          onClick={(e) => handleClick(e, email)}
                          className="text-left text-xs px-1 py-0.5 rounded-sm hover:bg-accent hover:text-accent-foreground hover:underline transition-colors"
                        >
                          {email}
                        </button>
                      ))}
                    </div>
                  </TooltipContent>
                </Tooltip>
              )}
            </div>
          </div>
        );
      },
      meta: { className: 'text-left text-xs' }
    },
    {
      accessorKey: "subject",
      header: t('search.subject'),
      cell: ({ row }) => {
        const tags = row.original.tags ?? [];
        return (
          <div className="flex flex-col gap-1 py-1.5 min-w-0">
            <LongText className='text-xs font-medium truncate'>
              {row.original.subject}
            </LongText>
            {tags.length > 0 && (
              <div className="flex items-center gap-1 flex-wrap">
                {tags.map((tag) => (
                  <span
                    key={tag}
                    className="inline-flex items-center rounded-md bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary ring-1 ring-inset ring-primary/20"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            )}
          </div>
        );
      },
      meta: { className: 'text-left text-xs' },
      minSize: 500,
      maxSize: 500,
    },
    {
      id: "text_preview",
      header: t('search.preview'),
      cell: ({ row }) => {
        const preview = row.original.preview

        if (!preview) return null

        return (
          <HoverCard openDelay={200} closeDelay={150}>
            <HoverCardTrigger asChild>
              <button
                type="button"
                className="text-muted-foreground hover:text-primary transition"
                onClick={(e) => e.stopPropagation()}
              >
                <MessageSquareText size={16} />
              </button>
            </HoverCardTrigger>

            <HoverCardContent
              side="right"
              align="start"
              className="max-w-[520px] max-h-[420px] overflow-auto whitespace-pre-wrap text-xs leading-relaxed"
            >
              {preview}
            </HoverCardContent>
          </HoverCard>
        )
      },
      meta: { className: "text-center text-xs" },
      enableSorting: false,
      minSize: 100,
      maxSize: 100,
    },
    {
      id: "attachment_count",
      header: () => <Paperclip size={16} />,
      cell: ({ row }) => <span className='text-xs'>{row.original.regular_attachment_count}</span>,
      meta: { className: 'text-left text-xs' },
      minSize: 30,
      maxSize: 30,
    },
    {
      accessorKey: 'size',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('search.size')} />
      ),
      cell: ({ row }) => <span className='text-xs max-w-[40px]'>{formatBytes(row.original.size)}</span>,
      meta: { className: 'text-left text-xs' },
      minSize: 80,
      maxSize: 80,
    },
    {
      accessorKey: 'date',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('search.date')} />
      ),
      cell: ({ row }) => {
        const date = new Date(row.original.date)
        const title = format(date, 'yyyy-MM-dd HH:mm:ss')
        return (
          <Tooltip>
            <TooltipTrigger asChild>
              <span className='text-xs whitespace-nowrap'>
                {formatDistanceToNow(date, { addSuffix: true, locale })}
              </span>
            </TooltipTrigger>
            <TooltipContent>{title}</TooltipContent>
          </Tooltip>
        )
      },
      meta: { className: 'text-left text-xs' },
      minSize: 130,
      maxSize: 130,
    },
    {
      id: 'actions',
      header: t('users.columns.actions'),
      cell: DataTableRowActions,
      meta: { className: 'text-center text-xs' },
      minSize: 60,
      maxSize: 60,
    },
  ]

  return (
    <MailListDataTable
      items={items}
      isLoading={isLoading}
      columns={columns}
      setSortBy={setSortBy}
      setSortOrder={setSortOrder}
      onRowClick={(e, row) => {
        const target = e.target as HTMLElement
        if (target.closest('input[type="checkbox"], button')) return
        onEnvelopeChanged(row.original)
      }}
      toolbar={(table) => <DataTableToolbar table={table} />}
      bulkActions={<MailBulkActions />}
    />
  )
}
