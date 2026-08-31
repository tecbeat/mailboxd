import { fireEvent, screen, waitFor } from '@testing-library/react';
import { delay, http, HttpResponse } from 'msw';
import { describe, expect, it } from 'vitest';

import { renderWithProviders } from '@/test/utils';
import { server } from '@/test/server';

import { ProxyDeleteDialog } from '../delete-dialog';

const BASE = 'http://localhost:15630';
const proxy = { id: 1, url: 'http://proxy', created_at: 0, updated_at: 0 };

describe('ProxyDeleteDialog double-submit guard', () => {
  it('disables the confirm button while the delete is in flight', async () => {
    server.use(
      http.delete(`${BASE}/api/v1/proxy/:id`, async () => {
        await delay(100);
        return HttpResponse.json({});
      }),
    );

    renderWithProviders(<ProxyDeleteDialog open onOpenChange={() => {}} currentRow={proxy} />);

    // Confirm is gated behind typing the proxy id.
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '1' } });
    const confirm = screen.getByRole('button', { name: 'Delete' });
    expect(confirm).toBeEnabled();

    fireEvent.click(confirm);

    await waitFor(() => expect(confirm).toBeDisabled());
  });
});
