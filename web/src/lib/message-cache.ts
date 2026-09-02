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
