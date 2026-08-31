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
