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

import { invalidateAccessTokenViews, invalidateAccountAccessViews } from '../access-cache';

describe('invalidateAccessTokenViews', () => {
  it('invalidates the global token list and the per-user token list', () => {
    const qc = new QueryClient();
    qc.setQueryData(['access-token-list'], []);
    qc.setQueryData(['user-tokens', 7], []);
    qc.setQueryData(['user-tokens', 8], []);

    invalidateAccessTokenViews(qc, 7);

    expect(qc.getQueryState(['access-token-list'])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(['user-tokens', 7])?.isInvalidated).toBe(true);
    // A different user's token list must not be touched.
    expect(qc.getQueryState(['user-tokens', 8])?.isInvalidated).toBe(false);
  });
});

describe('invalidateAccountAccessViews', () => {
  it('invalidates the caches that render a user account access map', () => {
    const qc = new QueryClient();
    qc.setQueryData(['user-list'], []);
    qc.setQueryData(['current-user'], {});

    invalidateAccountAccessViews(qc);

    expect(qc.getQueryState(['user-list'])?.isInvalidated).toBe(true);
    expect(qc.getQueryState(['current-user'])?.isInvalidated).toBe(true);
  });
});
