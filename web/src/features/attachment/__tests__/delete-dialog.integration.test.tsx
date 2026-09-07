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

import AttachmentProvider from '../context/provider';
import { useAttachmentContext } from '../context';
import { EnvelopeDeleteDialog } from '@/features/mail-list/delete-dialog';
import { MailListConfigProvider } from '@/features/mail-list/config-provider';

const BASE = 'http://localhost:15630';

type AttachmentContextValue = ReturnType<typeof useAttachmentContext>;

function makeContext(overrides: Partial<AttachmentContextValue> = {}): AttachmentContextValue {
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
    ...overrides,
  };
}

describe('attachment delete dialog cross-view invalidation', () => {
  it('refreshes the search view after deleting from the attachment view', async () => {
    server.use(
      http.post(`${BASE}/api/v1/delete-messages`, () => HttpResponse.json({ success: true })),
    );

    const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    queryClient.setQueryData(['search-messages', 1, { q: 'x' }], []);

    renderWithProviders(
      <AttachmentProvider value={makeContext({ open: 'delete', toDelete: new Map([[1, new Set(['msg-1'])]]) })}>
        <MailListConfigProvider config={{ useListContext: useAttachmentContext, useCurrentEnvelope: () => ({ data: undefined, isLoading: false, error: null }) }}>
          <EnvelopeDeleteDialog open onOpenChange={() => {}} />
        </MailListConfigProvider>
      </AttachmentProvider>,
      { queryClient },
    );

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(queryClient.getQueryState(['search-messages', 1, { q: 'x' }])?.isInvalidated).toBe(true),
    );
  });
});
