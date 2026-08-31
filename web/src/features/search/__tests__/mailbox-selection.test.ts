import { describe, expect, it } from 'vitest';

import { countSelectedMailboxes } from '../mailbox-selection';

describe('countSelectedMailboxes', () => {
  const mailboxes = [{ id: 1 }, { id: 2 }, { id: 3 }];

  it('counts how many mailboxes are selected', () => {
    expect(countSelectedMailboxes(mailboxes, new Set([1, 3]))).toBe(2);
  });

  it('ignores selected ids that are not in the mailbox list', () => {
    expect(countSelectedMailboxes(mailboxes, new Set([3, 99]))).toBe(1);
  });

  it('returns 0 for missing data or an empty selection', () => {
    expect(countSelectedMailboxes(undefined, new Set([1]))).toBe(0);
    expect(countSelectedMailboxes(mailboxes, new Set())).toBe(0);
  });
});
