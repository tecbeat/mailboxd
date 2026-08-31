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

import { fireEvent, screen } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { describe, expect, it, vi } from 'vitest';

import { fileOfSize } from '@/test/files';
import { renderWithProviders } from '@/test/utils';
import { makeSysConfig, server } from '@/test/server';

// The shared page header pulls in router/profile chrome unrelated to import logic.
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }));

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
