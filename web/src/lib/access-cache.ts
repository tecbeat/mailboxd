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
