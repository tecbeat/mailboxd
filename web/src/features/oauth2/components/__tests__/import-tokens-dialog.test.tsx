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

import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import type { MinimalAccount } from '@/api/account/api';
import { server } from '@/test/server';
import { renderWithProviders } from '@/test/utils';

import type { OAuth2Entity } from '../../data/schema';
import { ImportTokensDialog } from '../import-tokens-dialog';

const { navigate, toast } = vi.hoisted(() => ({ navigate: vi.fn(), toast: vi.fn() }));
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }));
vi.mock('@/hooks/use-toast', () => ({ toast }));

// The virtualized picker renders no rows in jsdom; a native select with the
// same props keeps the test on the dialog's own behaviour.
vi.mock('@/components/virtualized-select', () => ({
  VirtualizedSelect: ({
    options,
    onSelectOption,
    placeholder,
  }: {
    options: { label: string; value: string }[];
    onSelectOption?: (values: string[]) => void;
    placeholder: string;
  }) => (
    <select aria-label={placeholder} onChange={(e) => onSelectOption?.([e.target.value])}>
      <option value=''>{placeholder}</option>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  ),
}));

// The user may manage accounts 1 and 2, but not 4.
const MANAGED = new Set([1, 2]);
vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({
    require_any_permission: (_: string[], accountId?: number) =>
      accountId === undefined || MANAGED.has(accountId),
  }),
}));

const BASE = 'http://localhost:15630';
const entity = { id: 9, client_id: 'client-9' } as unknown as OAuth2Entity;

const ACCOUNTS: MinimalAccount[] = [
  { id: 1, email: 'oauth@example.com', auth_type: 'OAuth2' },
  { id: 2, email: 'password@example.com', auth_type: 'Password' },
  { id: 3, email: 'nosync@example.com', auth_type: null },
  { id: 4, email: 'foreign@example.com', auth_type: 'OAuth2' },
];

function useAccounts(accounts: MinimalAccount[]) {
  server.use(http.get(`${BASE}/api/v1/minimal-account-list`, () => HttpResponse.json(accounts)));
}

describe('ImportTokensDialog', () => {
  it('offers only OAuth2 accounts the user may manage', async () => {
    useAccounts(ACCOUNTS);
    renderWithProviders(<ImportTokensDialog open onOpenChange={() => {}} currentRow={entity} />);

    const picker = await screen.findByRole('combobox', { name: 'Select an account' });
    const offered = Array.from((picker as HTMLSelectElement).options, (o) => o.text).slice(1);
    expect(offered).toEqual(['oauth@example.com']);
  });

  it('explains when no account uses OAuth2', async () => {
    useAccounts(ACCOUNTS.slice(1, 3));
    renderWithProviders(<ImportTokensDialog open onOpenChange={() => {}} currentRow={entity} />);

    expect(await screen.findByText(/None of your accounts use OAuth2/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Import Tokens' })).not.toBeInTheDocument();
  });

  it('stores the trimmed tokens for the chosen account and this configuration', async () => {
    useAccounts(ACCOUNTS);
    let stored: { accountId: string; body: unknown } | undefined;
    server.use(
      http.post(`${BASE}/api/v1/store-external-oauth2-token/:accountId`, async ({ params, request }) => {
        stored = { accountId: params.accountId as string, body: await request.json() };
        return HttpResponse.json(null);
      }),
    );
    const onOpenChange = vi.fn();
    renderWithProviders(<ImportTokensDialog open onOpenChange={onOpenChange} currentRow={entity} />);

    await userEvent.selectOptions(
      await screen.findByRole('combobox', { name: 'Select an account' }),
      'oauth@example.com',
    );
    await userEvent.type(screen.getByPlaceholderText('Paste the access token here.'), '  ya29.access  ');
    await userEvent.type(screen.getByPlaceholderText('Paste the refresh token here.'), '1//refresh');
    await userEvent.click(screen.getByRole('button', { name: 'Import Tokens' }));

    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
    expect(stored).toEqual({
      accountId: '1',
      body: { oauth2_id: 9, access_token: 'ya29.access', refresh_token: '1//refresh' },
    });
  });

  it('requires an account and both tokens', async () => {
    useAccounts(ACCOUNTS);
    renderWithProviders(<ImportTokensDialog open onOpenChange={() => {}} currentRow={entity} />);

    await userEvent.click(await screen.findByRole('button', { name: 'Import Tokens' }));

    expect(await screen.findByText('Please select an account.')).toBeInTheDocument();
    expect(screen.getByText('Access Token cannot be empty')).toBeInTheDocument();
    expect(screen.getByText('Refresh Token cannot be empty')).toBeInTheDocument();
  });

  it('shows the server message when the import fails', async () => {
    useAccounts(ACCOUNTS);
    server.use(
      http.post(`${BASE}/api/v1/store-external-oauth2-token/:accountId`, () =>
        HttpResponse.json({ message: 'refresh token rejected' }, { status: 400 }),
      ),
    );
    const onOpenChange = vi.fn();
    renderWithProviders(<ImportTokensDialog open onOpenChange={onOpenChange} currentRow={entity} />);

    await userEvent.selectOptions(
      await screen.findByRole('combobox', { name: 'Select an account' }),
      'oauth@example.com',
    );
    await userEvent.type(screen.getByPlaceholderText('Paste the access token here.'), 'a');
    await userEvent.type(screen.getByPlaceholderText('Paste the refresh token here.'), 'r');
    await userEvent.click(screen.getByRole('button', { name: 'Import Tokens' }));

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: 'destructive',
          title: 'Failed to import tokens',
          description: 'refresh token rejected',
        }),
      ),
    );
    expect(onOpenChange).not.toHaveBeenCalled();
  });

  it('clears the form when the dialog is closed', async () => {
    useAccounts(ACCOUNTS);
    const onOpenChange = vi.fn();
    renderWithProviders(<ImportTokensDialog open onOpenChange={onOpenChange} currentRow={entity} />);

    const access = await screen.findByPlaceholderText('Paste the access token here.');
    await userEvent.type(access, 'half-typed');
    await userEvent.keyboard('{Escape}');

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(access).toHaveValue('');
  });

  it('sends users without any account to create one', async () => {
    useAccounts([]);
    renderWithProviders(<ImportTokensDialog open onOpenChange={() => {}} currentRow={entity} />);

    expect(await screen.findByText(/No email accounts registered/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Create Account' }));

    expect(navigate).toHaveBeenCalledWith({ to: '/accounts' });
  });
});
