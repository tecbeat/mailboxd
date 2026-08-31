import { QueryClient } from '@tanstack/react-query';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import SearchProvider, { useSearchContext } from '../context';
import { EnvelopeDeleteDialog } from '../delete-dialog';

const BASE = 'http://localhost:15630';

type SearchContextValue = ReturnType<typeof useSearchContext>;

function makeContext(overrides: Partial<SearchContextValue> = {}): SearchContextValue {
  return {
    open: null,
    setOpen: vi.fn(),
    currentEnvelope: undefined,
    setCurrentEnvelope: vi.fn(),
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
        <EnvelopeDeleteDialog open onOpenChange={() => {}} />
      </SearchProvider>,
      { queryClient },
    );

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(queryClient.getQueryState(['search-attachments', 1, { q: 'x' }])?.isInvalidated).toBe(true),
    );
  });
});
