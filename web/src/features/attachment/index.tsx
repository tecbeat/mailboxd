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
import { DataTablePagination } from '@/components/data-table/data-table-pagination';
import React from 'react';
import AttachmentProvider, { AttachmentDialogType, useAttachmentContext } from './context';
import { MailListConfigProvider, type MailListConfig } from '@/features/mail-list/config';
import { useEnvelope } from '@/hooks/use-envelope';
import useDialogState from '@/hooks/use-dialog-state';
import { useTranslation } from 'react-i18next';
import { AttachmentListTable } from './mail-list-table';
import { SortingState } from '@tanstack/react-table';
import { useSearchAttachments } from '@/hooks/use-search-attachments';
import { AttachmentModel } from '@/api/attachment/api';
import { AttachmentDialogs } from './dialogs';

// Attachment's list item only references an envelope, so it must be fetched.
function useAttachmentCurrentEnvelope() {
  const { currentItem } = useAttachmentContext()
  return useEnvelope(currentItem?.account_id, currentItem?.envelope_id)
}

const ATTACHMENT_LIST_CONFIG: MailListConfig = {
  useListContext: useAttachmentContext,
  useCurrentEnvelope: useAttachmentCurrentEnvelope,
}

export default function AttachmentSearch() {
  const { t } = useTranslation()
  const [currentItem, setCurrentItem] = React.useState<AttachmentModel | undefined>(undefined);
  const [open, setOpen] = useDialogState<AttachmentDialogType>(null)
  const [toDelete, setToDelete] = React.useState<Map<number, Set<string>>>(new Map());
  const [selected, setSelected] = React.useState<Map<number, Set<string>>>(new Map());
  const [selectedTags, setSelectedTags] = React.useState<string[]>([]);
  const [sorting, setSorting] = React.useState<SortingState>([{ id: "date", desc: true }]);
  const [deleteMailboxId, setDeleteMailboxId] = React.useState<string | undefined>(undefined);
  const [selectedAccountId, setSelectedAccountId] = React.useState<number | undefined>(undefined);

  const {
    attachments,
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
  } = useSearchAttachments();

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
      <Main>
        <AttachmentProvider
          value={{
            open,
            setOpen,
            currentItem,
            selectedTags,
            setCurrentItem,
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
            handleTagToggle
          }}
        >
          <MailListConfigProvider config={ATTACHMENT_LIST_CONFIG}>
          <div>
            <PageHeader
              className="mb-4"
              title={t('attachment.title', 'Attachments')}
              description={t('attachment.description', 'Search and browse archived email attachments.')}
            />
            <div className="flex gap-6">
              <div className="flex-1 min-w-0 space-y-4">
                {isLoading && (
                  <Card>
                    <CardContent className="py-12">
                      <div className="flex flex-col items-center gap-2 text-muted-foreground">
                        <Spinner />
                        <p className="text-sm">{t('search.searching')}</p>
                      </div>
                    </CardContent>
                  </Card>
                )}

                <AttachmentListTable
                  isLoading={isLoading}
                  items={attachments}
                  setSortBy={setSortBy}
                  setSortOrder={setSortOrder}
                />
                {total > 0 && <DataTablePagination
                  storageKey='attachment'
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
                />}
              </div>
            </div>
          </div>

          <AttachmentDialogs />
          </MailListConfigProvider>
        </AttachmentProvider>
      </Main>
    </>
  );
}
