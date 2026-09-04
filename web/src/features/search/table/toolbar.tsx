import { type RowData, type Table } from '@tanstack/react-table'
import { type MailTableFeatures } from '@/lib/data-table'
import { DataTableViewOptions } from '@/features/mail-list/table/view-options'
import { TagFilterPopover } from '../tag-filter-popover'
import { TimePopover } from '@/features/mail-list/time-popover'
import { MailFilterPopover } from '../contact-popover'
import { TextSearchInput, type TextSearchConfig } from '@/features/mail-list/text-search-input'
import { MoreFiltersPopover } from '../more-filters-popover'
import { FilterResetButton } from '@/features/mail-list/filter-reset'
import { MailboxPopover } from '@/features/mail-list/mailbox-popover'
import { AccountPopover } from '@/features/mail-list/account-popover'

const SEARCH_TEXT_CONFIG: TextSearchConfig = {
  storageKey: 'mailboxd_mail_search_history',
  searchFields: ['text', 'subject', 'body'],
  placeholderKey: 'search_input.placeholder',
  options: [
    { value: 'text', labelKey: 'search_input.all', descKey: 'search_input.all_fields_desc' },
    { value: 'subject', labelKey: 'search_input.subject' },
    { value: 'body', labelKey: 'search_input.body' },
  ],
}

type DataTableToolbarProps<TData extends RowData> = {
  table: Table<MailTableFeatures, TData>
}

export function DataTableToolbar<TData extends RowData>({
  table,
}: DataTableToolbarProps<TData>) {
  return (
    <div className="flex flex-col gap-2 sm:flex-row sm:items-center p-1 bg-background">
      <div className="w-full sm:w-auto sm:flex-1 sm:max-w-[620px]">
        <TextSearchInput config={SEARCH_TEXT_CONFIG} />
      </div>
      <div className="flex items-center gap-2 w-full sm:flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-1 min-w-0 overflow-x-auto scrollbar-thin">
          <div className="flex items-center gap-1.5 shrink-0">
            <AccountPopover />
            <MailboxPopover />
            <MailFilterPopover />
            <TagFilterPopover />
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