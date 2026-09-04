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
import { FilterBar } from '@/features/mail-list/filter-bar'
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
    <div className="p-1 bg-background">
      <FilterBar dense>
        <TextSearchInput config={ATTACHMENT_TEXT_CONFIG} />
        <AccountPopover />
        <MailboxPopover />
        <SenderFilterPopover />
        <MetadataFilter
          type="extension"
          icon={<FileType className="h-4 w-4" />}
        />
        <MetadataFilter
          type="category"
          icon={<Tag className="h-4 w-4" />}
        />
        <MetadataFilter
          type="content_type"
          icon={<Laptop className="h-4 w-4" />}
        />
        <MoreFiltersPopover />
        <FilterResetButton />
        <TimePopover />
        <DataTableViewOptions table={table} />
      </FilterBar>
    </div>
  )
}