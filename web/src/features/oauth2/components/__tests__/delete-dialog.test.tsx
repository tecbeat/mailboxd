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
import { delay, http, HttpResponse } from 'msw';
import { describe, expect, it } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import type { OAuth2Entity } from '../../data/schema';
import { TokenDeleteDialog } from '../delete-dialog';

const BASE = 'http://localhost:15630';
const entity = { id: 1, client_id: 'client-1' } as unknown as OAuth2Entity;

describe('OAuth2 TokenDeleteDialog double-submit guard', () => {
  it('disables the confirm button while the delete is in flight', async () => {
    server.use(
      http.delete(`${BASE}/api/v1/oauth2/:id`, async () => {
        await delay(100);
        return HttpResponse.json({});
      }),
    );

    renderWithProviders(<TokenDeleteDialog open onOpenChange={() => {}} currentRow={entity} />);

    // Confirm is gated behind typing the OAuth2 id.
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '1' } });
    const confirm = screen.getByRole('button', { name: 'Delete' });
    expect(confirm).toBeEnabled();

    fireEvent.click(confirm);

    await waitFor(() => expect(confirm).toBeDisabled());
  });
});
