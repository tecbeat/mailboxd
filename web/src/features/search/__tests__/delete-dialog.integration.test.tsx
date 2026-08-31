//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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

import { QueryClient } from '@tanstack/react-query';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import SearchProvider, { useSearchContext } from '../context';
import { EnvelopeDeleteDialog } from '@/features/mail-list/delete-dialog';
import { MailListConfigProvider } from '@/features/mail-list/config';

const BASE = 'http://localhost:15630';

type SearchContextValue = ReturnType<typeof useSearchContext>;

function makeContext(overrides: Partial<SearchContextValue> = {}): SearchContextValue {
  return {
    open: null,
    setOpen: vi.fn(),
    currentItem: undefined,
    setCurrentItem: vi.fn(),
    toDelete: new Map(),
    setToDelete: vi.fn(),
    selected: new Map(),
    setSelected: vi.fn(),
    deleteMailboxId: undefined,
    setDeleteMailboxId: vi.fn(),
    selectedAccountId: undefined,
    setSelectedAccountId: vi.fn(),
    selectedTags: [],
    sorting: [],
    setSorting: vi.fn(),
    filter: {},
    setFilter: vi.fn(),
    handleTagToggle: vi.fn(),
    editTagsOpen: false,
    setEditTagsOpen: vi.fn(),
    ...overrides,
  };
}

describe('search delete dialog cross-view invalidation', () => {
  it('refreshes the attachment view after deleting from the search view', async () => {
    server.use(
      http.post(`${BASE}/api/v1/delete-messages`, () => HttpResponse.json({ success: true })),
    );

    const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    queryClient.setQueryData(['search-attachments', 1, { q: 'x' }], []);

    renderWithProviders(
      <SearchProvider value={makeContext({ open: 'delete', toDelete: new Map([[1, new Set(['msg-1'])]]) })}>
        <MailListConfigProvider config={{ useListContext: useSearchContext, useCurrentEnvelope: () => ({ data: undefined, isLoading: false, error: null }) }}>
          <EnvelopeDeleteDialog open onOpenChange={() => {}} />
        </MailListConfigProvider>
      </SearchProvider>,
      { queryClient },
    );

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(queryClient.getQueryState(['search-attachments', 1, { q: 'x' }])?.isInvalidated).toBe(true),
    );
  });
});
