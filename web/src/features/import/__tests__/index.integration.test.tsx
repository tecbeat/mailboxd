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
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { beforeAll, describe, expect, it, vi } from 'vitest';

import { fileOfSize } from '@/test/files';
import { renderWithProviders } from '@/test/utils';
import { makeSysConfig, server } from '@/test/server';

// The shared page header pulls in router/profile chrome unrelated to import logic.
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }));
const { toast } = vi.hoisted(() => ({ toast: vi.fn() }));
vi.mock('@/hooks/use-toast', () => ({ toast }));

import ImportPage from '../index';

const BASE = 'http://localhost:15630';
const MB = 1024 * 1024;

describe('ImportPage upload limits', () => {
  it('validates a dropped file against the server-configured limit, not the default', async () => {
    // Server allows only 10 MB mbox uploads — well below the 1 GB in-code default.
    server.use(
      http.get(`${BASE}/api/v1/system-configurations`, () =>
        HttpResponse.json(makeSysConfig({ mailboxd_web_mbox_upload_limit_mb: 10 })),
      ),
    );

    renderWithProviders(<ImportPage />);

    // The rendered limit reflects the server value once the query resolves.
    await screen.findByText(/MBOX 10 MB/);

    const dropzone = screen.getByText(/Drop .* files here/i).closest('div')!;
    const bigMbox = fileOfSize('inbox.mbox', 50 * MB); // 50 MB > 10 MB server limit
    fireEvent.drop(dropzone, { dataTransfer: { files: [bigMbox] } });

    // Must be flagged too large. The stale-closure bug captured the 1 GB default
    // at first render and wrongly accepted the file.
    expect(await screen.findByText('Too large')).toBeInTheDocument();
  });
});

describe('ImportPage PST selection', () => {
  it('accepts a PST that the browser reports as application/vnd.ms-outlook', async () => {
    renderWithProviders(<ImportPage />);

    const dropzone = (await screen.findByText(/Drop .* files here/i)).closest('div')!;
    const pst = new File([new Uint8Array(16)], 'backup.pst', { type: 'application/vnd.ms-outlook' });
    fireEvent.drop(dropzone, { dataTransfer: { files: [pst] } });

    expect(await screen.findByText(/PST folder structure will be preserved/)).toBeInTheDocument();
    expect(screen.queryByText('Invalid type')).not.toBeInTheDocument();
  });
});

describe('ImportPage import run', () => {
  beforeAll(() => {
    // jsdom lacks the layout APIs the account picker (cmdk) uses.
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );
    Element.prototype.scrollIntoView ??= () => {};
  });

  function serveAccount() {
    server.use(
      http.get(`${BASE}/api/v1/accounts`, () =>
        HttpResponse.json({
          items: [
            {
              id: 7,
              account_type: 'NoSync',
              enabled: true,
              account_name: 'Archive',
              email: 'archive@example.com',
              download_folders: [],
              created_by: 1,
              created_user_name: 'admin',
              created_user_email: 'admin@example.com',
              created_at: 0,
              updated_at: 0,
              use_dangerous: false,
            },
          ],
        }),
      ),
      http.get(`${BASE}/api/v1/list-mailboxes/7`, () => HttpResponse.json([])),
    );
  }

  async function importOneEml() {
    renderWithProviders(<ImportPage />);
    await userEvent.click(await screen.findByRole('combobox'));
    await userEvent.click(await screen.findByRole('option', { name: /Archive/ }));

    const dropzone = (await screen.findByText(/Drop .* files here/i)).closest('div')!;
    const eml = new File(['Subject: hi\r\n\r\nbody'], 'mail.eml', { type: 'message/rfc822' });
    fireEvent.drop(dropzone, { dataTransfer: { files: [eml] } });

    const start = await screen.findByRole('button', { name: 'Import' });
    await waitFor(() => expect(start).toBeEnabled());
    await userEvent.click(start);
  }

  it('uploads into the chosen account and reports skipped duplicates', async () => {
    serveAccount();
    let uploaded: URLSearchParams | undefined;
    server.use(
      http.post(`${BASE}/api/v1/upload-import`, ({ request }) => {
        uploaded = new URL(request.url).searchParams;
        return HttpResponse.json({
          import_id: 'imp-1',
          status: 'Completed',
          format: 'eml',
          total: 1,
          success: 0,
          duplicates: 1,
          failed: 0,
          failed_details: [],
        });
      }),
    );
    await importOneEml();

    expect(await screen.findByText(/1 duplicate/i)).toBeInTheDocument();
    expect(uploaded?.get('account_id')).toBe('7');
    expect(uploaded?.get('file_name')).toBe('mail.eml');
  });

  it('reports mail skipped as outside the retention period', async () => {
    serveAccount();
    server.use(
      http.post(`${BASE}/api/v1/upload-import`, () =>
        HttpResponse.json({
          import_id: 'imp-2',
          status: 'Completed',
          format: 'eml',
          total: 3,
          success: 1,
          duplicates: 0,
          outside_retention: 2,
          failed: 0,
          failed_details: [],
        }),
      ),
    );
    await importOneEml();

    expect(await screen.findByText('2 outside the retention period')).toBeInTheDocument();
    // Skipped mail counts as processed, so the run reads as finished.
    expect(screen.getByText('3 / 3 processed')).toBeInTheDocument();
  });

  it('resets and explains when the upload fails', async () => {
    serveAccount();
    server.use(
      http.post(`${BASE}/api/v1/upload-import`, () =>
        HttpResponse.json({ message: 'disk full' }, { status: 507 }),
      ),
    );
    await importOneEml();

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({ variant: 'destructive', description: 'mail.eml: disk full' }),
      ),
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'Import' })).toBeEnabled());
  });
});

describe('ImportPage history', () => {
  it('lists mail skipped as outside the retention period', async () => {
    server.use(
      http.get(`${BASE}/api/v1/import-history`, () =>
        HttpResponse.json([
          {
            id: 'h-1',
            user_id: 1,
            import_id: 'imp-1',
            account_id: 7,
            folder: 'Archive',
            format: 'mbox',
            status: 'completed',
            total: 5,
            success: 1,
            duplicates: 0,
            outside_retention: 4,
            failed: 0,
            failed_details: [],
            created_at: 0,
          },
        ]),
      ),
    );
    renderWithProviders(<ImportPage />);

    await userEvent.click(await screen.findByRole('button', { name: /Import history/i }));
    expect(await screen.findByText('4 outside retention')).toBeInTheDocument();
  });
});
