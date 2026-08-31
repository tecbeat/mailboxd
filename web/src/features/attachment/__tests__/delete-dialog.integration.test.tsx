import { QueryClient } from '@tanstack/react-query';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import AttachmentProvider, { useAttachmentContext } from '../context';
import { EnvelopeDeleteDialog } from '../delete-dialog';

const BASE = 'http://localhost:15630';

type AttachmentContextValue = ReturnType<typeof useAttachmentContext>;

function makeContext(overrides: Partial<AttachmentContextValue> = {}): AttachmentContextValue {
  return {
    open: null,
    setOpen: vi.fn(),
    currentAttachment: undefined,
    setCurrentAttachment: vi.fn(),
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
        <EnvelopeDeleteDialog open onOpenChange={() => {}} />
      </AttachmentProvider>,
      { queryClient },
    );

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() =>
      expect(queryClient.getQueryState(['search-messages', 1, { q: 'x' }])?.isInvalidated).toBe(true),
    );
  });
});
