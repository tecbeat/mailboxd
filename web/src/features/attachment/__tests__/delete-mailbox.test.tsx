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

import { fireEvent, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import AttachmentProvider, { useAttachmentContext } from '../context';
import { AttachmentDialogs } from '../dialogs';

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

function renderDialogs(overrides: Partial<AttachmentContextValue>) {
  return renderWithProviders(
    <AttachmentProvider value={makeContext(overrides)}>
      <AttachmentDialogs />
    </AttachmentProvider>,
  );
}

describe('attachment delete-mailbox dialog', () => {
  it('renders the confirm dialog when open is "delete-mailbox"', () => {
    renderDialogs({ open: 'delete-mailbox', deleteMailboxId: 'Archive', selectedAccountId: 1 });

    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
    expect(screen.getByText('Delete Mailbox Folder')).toBeInTheDocument();
  });

  it('deletes the mailbox for the selected account when confirmed', async () => {
    let deleted: { accountId?: string; mailboxId?: string } = {};
    server.use(
      http.delete(`${BASE}/api/v1/delete-mailbox/:accountId/:mailboxId`, ({ params }) => {
        deleted = {
          accountId: params.accountId as string,
          mailboxId: params.mailboxId as string,
        };
        return HttpResponse.json({ success: true });
      }),
    );

    renderDialogs({ open: 'delete-mailbox', deleteMailboxId: 'Archive', selectedAccountId: 1 });

    fireEvent.click(screen.getByRole('button', { name: /Delete Permanently/i }));

    await waitFor(() => expect(deleted).toEqual({ accountId: '1', mailboxId: 'Archive' }));
  });
});
