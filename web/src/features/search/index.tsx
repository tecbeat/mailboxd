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


import { Card, CardContent } from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import { FixedHeader } from '@/components/layout/fixed-header';
import { Main } from '@/components/layout/main';
import { PageHeader } from '@/components/layout/page-header';
import { useSearchMessages } from '@/hooks/use-search-messages';
import { DataTablePagination } from '@/components/data-table/data-table-pagination';
import React from 'react';
import { EmailEnvelope } from '@/api';
import { MailDisplayDrawer } from '@/features/mail-list/mail-display-dialog';
import { EnvelopeDeleteDialog } from '@/features/mail-list/delete-dialog';
import SearchProvider, { SearchDialogType, useSearchContext } from './context';
import { MailListConfigProvider, type MailListConfig } from '@/features/mail-list/config';
import useDialogState from '@/hooks/use-dialog-state';
import { EditTagsDialog } from './edit-tag-dialog';
import { useTranslation } from 'react-i18next';
import { RestoreMessageDialog } from '@/features/mail-list/restore-message-dialog';
import { MailListTable } from './mail-list-table';
import { SortingState } from '@tanstack/react-table';
import { MailBoxDeleteDialog } from '@/features/mail-list/delete-mailbox-dialog';
import { UpdateTagsDialog } from './bulk-add-tag-dialog';

// Search's list item already is a full envelope, so it resolves synchronously.
function useSearchCurrentEnvelope() {
  const { currentItem } = useSearchContext()
  return { data: currentItem, isLoading: false, error: null }
}

const SEARCH_LIST_CONFIG: MailListConfig = {
  useListContext: useSearchContext,
  useCurrentEnvelope: useSearchCurrentEnvelope,
}

export default function EmailSearch() {
  const { t } = useTranslation()
  const [selectedEnvelope, setSelectedEnvelope] = React.useState<EmailEnvelope | undefined>(undefined);
  const [open, setOpen] = useDialogState<SearchDialogType>(null)
  const [toDelete, setToDelete] = React.useState<Map<number, Set<string>>>(new Map());
  const [selected, setSelected] = React.useState<Map<number, Set<string>>>(new Map());
  const [selectedTags, setSelectedTags] = React.useState<string[]>([]);
  const [sorting, setSorting] = React.useState<SortingState>([{ id: "date", desc: true }]);
  const [deleteMailboxId, setDeleteMailboxId] = React.useState<string | undefined>(undefined);
  const [selectedAccountId, setSelectedAccountId] = React.useState<number | undefined>(undefined);
  const [editTagsOpen, setEditTagsOpen] = React.useState(false);

  const {
    emails,
    total,
    totalPages,
    isLoading,
    page,
    pageSize,
    setPage,
    setSearchPageSize,
    setSortBy,
    setSortOrder,
    filter,
    setFilter
  } = useSearchMessages();

  const handleSetPageSize = (pageSize: number) => {
    setPage(1);
    setSearchPageSize(pageSize)
  }

  const handleTagToggle = (tag: string) => {
    setSelectedTags(prev =>
      prev.includes(tag)
        ? prev.filter(t => t !== tag)
        : [...prev, tag]
    );
  };

  return (
    <>
      <FixedHeader />
      <Main fixed>
        <SearchProvider
          value={{
            open,
            setOpen,
            currentItem: selectedEnvelope,
            selectedTags,
            setCurrentItem: setSelectedEnvelope,
            toDelete,
            setToDelete,
            selected,
            setSelected,
            sorting,
            setSorting,
            filter,
            setFilter,
            deleteMailboxId,
            setDeleteMailboxId,
            selectedAccountId,
            setSelectedAccountId,
            handleTagToggle,
            editTagsOpen,
            setEditTagsOpen,
          }}
        >
          <MailListConfigProvider config={SEARCH_LIST_CONFIG}>
          <div className="flex h-full min-h-0 flex-col">
            <PageHeader
              className="mb-4 shrink-0"
              title={t('search.title')}
              description={t('search.description', 'Search your archived emails across all accounts.')}
            />
            <div className="flex min-h-0 flex-1 gap-6">
              <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-4">
                {isLoading && (
                  <Card className="shrink-0">
                    <CardContent className="py-12">
                      <div className="flex flex-col items-center gap-2 text-muted-foreground">
                        <Spinner />
                        <p className="text-sm">{t('search.searching')}</p>
                      </div>
                    </CardContent>
                  </Card>
                )}

                <MailListTable
                  isLoading={isLoading}
                  items={emails}
                  onEnvelopeChanged={(envelope) => {
                    setOpen('display');
                    setSelectedEnvelope(envelope);
                  }}
                  setSortBy={setSortBy}
                  setSortOrder={setSortOrder}
                />
                {total > 0 && <div className="shrink-0">
                  <DataTablePagination
                  storageKey='search'
                  pageIndex={page - 1}
                  pageCount={Math.max(1, Math.ceil(total / pageSize))}
                  pageSize={pageSize}
                  canPreviousPage={page > 1}
                  canNextPage={page < totalPages}
                  onFirst={() => setPage(1)}
                  onPrevious={() => setPage(page - 1)}
                  onNext={() => setPage(page + 1)}
                  onLast={() => setPage(totalPages)}
                  onPageSizeChange={handleSetPageSize}
                />
                </div>}
              </div>
            </div>
          </div>

          <MailDisplayDrawer
            key='search-mail-display'
            open={open === 'display'}
            onOpenChange={() => setOpen('display')}
          />

          <EnvelopeDeleteDialog
            key='delete-envelope'
            open={open === 'delete'}
            onOpenChange={() => setOpen('delete')}
          />

          <EditTagsDialog
            key='edit-tags-dialog'
            open={editTagsOpen}
            onOpenChange={setEditTagsOpen}
          />

          <UpdateTagsDialog
            key='update-tags-dialog'
            open={open === 'update-tags'}
            onOpenChange={() => setOpen('update-tags')}
          />

          <RestoreMessageDialog
            key='restore-mail-dialog'
            open={open === 'restore'}
            onOpenChange={() => setOpen('restore')}
          />

          <MailBoxDeleteDialog
            key='mailbox-delete'
            open={open === 'delete-mailbox'}
            onOpenChange={() => setOpen('delete-mailbox')}
          />
          </MailListConfigProvider>
        </SearchProvider>
      </Main>
    </>
  );
}
