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
import { describe, expect, it } from 'vitest';

import { invalidateMessageViews } from '../message-cache';

function seededClient() {
  const qc = new QueryClient();
  // Nested keys exercise the exact:false list invalidations.
  qc.setQueryData(['search-messages', 1, { q: 'a' }], []);
  qc.setQueryData(['search-attachments', 1, { q: 'a' }], []);
  qc.setQueryData(['all-tags'], []);
  qc.setQueryData(['attachment-tags'], []);
  return qc;
}

describe('invalidateMessageViews', () => {
  it('invalidates both the search and attachment message lists and their tag caches', () => {
    const qc = seededClient();

    invalidateMessageViews(qc);

    expect(qc.getQueryState(['search-messages', 1, { q: 'a' }])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(['search-attachments', 1, { q: 'a' }])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(['all-tags'])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(['attachment-tags'])?.isInvalidated).toBe(true);
  });
});
