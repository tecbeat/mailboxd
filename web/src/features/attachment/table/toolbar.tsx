import { type RowData, type Table } from '@tanstack/react-table'
import { type MailTableFeatures } from '@/lib/data-table'
import { DataTableViewOptions } from '@/features/mail-list/table/view-options'
import { TimePopover } from '@/features/mail-list/time-popover'
import { SenderFilterPopover } from '../sender-popover'
import { TextSearchInput, type TextSearchConfig } from '@/features/mail-list/text-search-input'
import { MoreFiltersPopover } from '../more-filters-popover'
import { FilterResetButton } from '@/features/mail-list/filter-reset'
import { MailboxPopover } from '@/features/mail-list/mailbox-popover'
import { AccountPopover } from '@/features/mail-list/account-popover'
import { MetadataFilter } from '../attachment-metadata-filter'
import { FileType, Laptop, Tag } from 'lucide-react'

const ATTACHMENT_TEXT_CONFIG: TextSearchConfig = {
  storageKey: 'mailboxd_attachment_search_history',
  searchFields: ['text', 'subject', 'attachment_name', 'from'],
  placeholderKey: 'attachment.search_input_placeholder',
  options: [
    { value: 'text', labelKey: 'search_input.all', descKey: 'attachment.all_fields_desc' },
    { value: 'subject', labelKey: 'search_input.subject' },
    { value: 'body', labelKey: 'attachment.name' },
  ],
}

type DataTableToolbarProps<TData extends RowData> = {
  table: Table<MailTableFeatures, TData>
}

export function DataTableToolbar<TData extends RowData>({
  table,
}: DataTableToolbarProps<TData>) {
  return (
    <div className="flex flex-col gap-1 p-1 bg-background">
      <div className="mb-4 flex items-center justify-center w-full">
        <div className="w-full max-w-3xl">
          <TextSearchInput config={ATTACHMENT_TEXT_CONFIG} />
        </div>
      </div>
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2 sm:gap-1">
        <div className="flex items-center gap-2 flex-wrap w-full sm:w-auto">
          <div className="flex items-center gap-1.5 flex-wrap">
            <AccountPopover />
            <MailboxPopover />
            <SenderFilterPopover />
            <MetadataFilter
              type="extension"
              icon={<FileType className="h-3.5 w-3.5" />}
            />
            <MetadataFilter
              type="category"
              icon={<Tag className="h-3.5 w-3.5" />}
            />
            <MetadataFilter
              type="content_type"
              icon={<Laptop className="h-3.5 w-3.5" />}
            />

            <MoreFiltersPopover />
          </div>
          <FilterResetButton />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <TimePopover />
          <DataTableViewOptions table={table} />
        </div>
      </div>
    </div>
  )
}