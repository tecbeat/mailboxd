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
import { useAttachmentContext } from "./context"
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
import { AttachmentModel } from "@/api/attachment/api"
import { useSearchAttachments } from "@/hooks/use-search-attachments"
import { AttachmentIcon } from "./attachment-icon"

interface MailListProps {
  items: AttachmentModel[]
  isLoading: boolean
  setSortBy: (sortBy: "DATE" | "SIZE") => void
  setSortOrder: (value: "desc" | "asc") => void
}

export function AttachmentListTable({
  items,
  isLoading,
  setSortBy,
  setSortOrder
}: MailListProps) {
  const { t, i18n } = useTranslation()

  const locale = dateFnsLocaleMap[i18n.language.toLowerCase()] ?? enUS
  const { setOpen, setCurrentItem } = useAttachmentContext()
  const { setFilter } = useSearchAttachments();

  const columns: ColumnDef<MailTableFeatures, AttachmentModel>[] = [
    {
      accessorKey: "source",
      header: t('attachment.source'),
      cell: ({ row }) => {
        const { from, account_email, mailbox_name, account_id, mailbox_id } = row.original;
        const accountPrefix = account_email.split('@')[0];

        return (
          <div className="flex flex-col py-1.5 min-w-0 group">
            <div
              className="cursor-pointer hover:text-primary transition-colors flex items-center gap-1.5"
              onClick={(e) => {
                e.stopPropagation();
                setFilter((prev) => ({ ...prev, from: from }));
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
                  setFilter((prev) => ({ ...prev, account_ids: [account_id], mailbox_ids: undefined }));
                }}
              >
                {accountPrefix}
              </span>

              <span className="shrink-0 opacity-40">/</span>
              <span
                className="truncate max-w-[70px] hover:text-primary cursor-pointer transition-colors"
                onClick={(e) => {
                  e.stopPropagation();
                  setFilter((prev) => ({ ...prev, account_ids: [account_id], mailbox_ids: [mailbox_id] }));
                }}
              >
                {mailbox_name}
              </span>
            </div>
          </div>
        );
      },
      meta: { className: 'text-left text-xs' }
    },
    {
      accessorKey: "subject",
      header: t('attachment.subject'),
      cell: ({ row }) => {
        return (
          <div className="group relative flex items-center w-full min-w-0 h-full px-2 overflow-hidden">
            <div className="absolute left-0 top-0 bottom-0 w-[2px] bg-primary opacity-0 group-hover:opacity-100 transition-opacity" />

            <div className="text-xs flex flex-wrap gap-x-1 min-w-0 flex-1">
              <span className="flex items-center">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setCurrentItem(row.original);
                    setOpen("display");
                  }}
                  className="hover:text-primary hover:underline transition-colors truncate"
                >
                  <LongText>{row.original.subject}</LongText>
                </button>
              </span>
            </div>
          </div>
        );
      },
      meta: { className: 'text-left text-xs' },
      minSize: 500,
      maxSize: 500,
    },
    {
      accessorKey: "name",
      header: t('attachment.name'),
      cell: ({ row }) => {
        const { name, content_type, is_message } = row.original;
        const safeName = name ?? "n/a";

        const shortContentType = content_type
          ? content_type.split('/').pop()?.toUpperCase().replace('X-', '')
          : "UNK";

        return (
          <div className="flex flex-col min-w-0 py-1">
            <div className="flex items-center gap-2.5">
              <AttachmentIcon
                contentType={content_type ?? ""}
                className="h-4 w-4 mt-0.5"
              />

              {is_message && <div className="group relative flex items-center w-full min-w-0 h-full px-2 overflow-hidden">
                <div className="absolute left-0 top-0 bottom-0 w-[2px] bg-primary opacity-0 group-hover:opacity-100 transition-opacity" />

                <div className="text-xs flex flex-wrap gap-x-1 min-w-0 flex-1">
                  <span className="flex items-center">
                    <button
                      type="button"
                      title={t('attachment.viewEmbeddedEmail')}
                      onClick={(e) => {
                        e.stopPropagation();
                        setCurrentItem(row.original);
                        setOpen("nested-eml");
                      }}
                      className="hover:text-primary hover:underline transition-colors truncate"
                    >
                      <LongText>{safeName}</LongText>
                    </button>
                  </span>
                </div>
              </div>}
              {!is_message && <LongText className='text-xs font-medium max-w-[320px] text-foreground/90'>
                {safeName}
              </LongText>}
            </div>
            <div className="flex items-center gap-1 ml-6.5 mt-1">
              <span className="text-[10px] text-muted-foreground font-mono bg-muted px-1 py-0.5 rounded-xs">
                {shortContentType}
              </span>
            </div>
          </div>
        );
      },
      meta: { className: 'text-left text-xs' },
    },
    {
      accessorKey: 'size',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('attachment.size')} />
      ),
      cell: ({ row }) => <span className='text-xs max-w-[40px]'>{formatBytes(row.original.size)}</span>,
      meta: { className: 'text-left text-xs' },
      minSize: 80,
      maxSize: 80,
    },
    {
      accessorKey: 'date',
      header: ({ column }) => (
        <DataTableColumnHeader column={column} title={t('attachment.date')} />
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
      meta: { className: 'text-left text-xs' },
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
      onRowClick={() => { }}
      toolbar={(table) => <DataTableToolbar table={table} />}
    />
  )
}
