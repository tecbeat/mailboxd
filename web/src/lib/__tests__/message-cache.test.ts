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
