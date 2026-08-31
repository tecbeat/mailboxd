//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.

import type { QueryClient } from '@tanstack/react-query';

/**
 * Invalidate every cache that shows archived messages or their tags.
 *
 * The search and attachment views both delete messages through the same
 * `delete_messages` call, so a delete in one must refresh the other; otherwise
 * the untouched view keeps showing messages that no longer exist.
 */
export function invalidateMessageViews(queryClient: QueryClient): void {
  queryClient.invalidateQueries({ queryKey: ['search-messages'], exact: false });
  queryClient.invalidateQueries({ queryKey: ['search-attachments'], exact: false });
  queryClient.invalidateQueries({ queryKey: ['all-tags'] });
  queryClient.invalidateQueries({ queryKey: ['attachment-tags'] });
}
