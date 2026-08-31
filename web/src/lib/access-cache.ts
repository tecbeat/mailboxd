//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.

import type { QueryClient } from '@tanstack/react-query';

/**
 * Invalidate the caches that list access tokens after a token is created or
 * updated: the global admin list (`access-token-list`) and the per-user list
 * (`user-tokens`). Shared by the settings and user-management token dialogs so
 * their invalidation cannot drift apart again.
 */
export function invalidateAccessTokenViews(queryClient: QueryClient, userId: number): void {
  queryClient.invalidateQueries({ queryKey: ['access-token-list'] });
  queryClient.invalidateQueries({ queryKey: ['user-tokens', userId] });
}

/**
 * Invalidate the caches that render a user's account access map after an access
 * assignment: the users list and the current user (the "My Access" view).
 */
export function invalidateAccountAccessViews(queryClient: QueryClient): void {
  queryClient.invalidateQueries({ queryKey: ['user-list'] });
  queryClient.invalidateQueries({ queryKey: ['current-user'] });
}
