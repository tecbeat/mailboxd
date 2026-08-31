//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.

/**
 * Count how many of the given mailboxes are selected.
 *
 * Callers pass a Set so the lookup is O(1); this keeps the per-account badge
 * computation O(mailboxes) instead of O(mailboxes x selectedIds) on every
 * render.
 */
export function countSelectedMailboxes(
  mailboxes: readonly { id: number }[] | undefined,
  selectedIds: ReadonlySet<number>,
): number {
  if (!mailboxes) return 0;
  let count = 0;
  for (const mailbox of mailboxes) {
    if (selectedIds.has(mailbox.id)) count++;
  }
  return count;
}
