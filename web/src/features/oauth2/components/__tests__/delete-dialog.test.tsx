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
