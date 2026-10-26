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

import { type SortingState } from '@tanstack/react-table'

export type ListSortBy = 'DATE' | 'SIZE' | 'RELEVANCE'

export const DEFAULT_SORTING: SortingState = [{ id: 'date', desc: true }]

/**
 * The table sorting for the sort in the URL. Relevance has no column, so an
 * empty table sorting means "relevance": the page keeps the server's order
 * and no column header shows a sort.
 */
export function initialSorting(sortBy: ListSortBy, sortOrder: 'asc' | 'desc'): SortingState {
  if (sortBy === 'RELEVANCE') return []
  return [{ id: sortBy.toLowerCase(), desc: sortOrder === 'desc' }]
}

/** The search parameters for a table sorting; relevance has no direction. */
export function sortParams(sorting: SortingState): { sortBy: ListSortBy; sortOrder?: 'asc' | 'desc' } {
  const [value] = sorting
  if (!value) return { sortBy: 'RELEVANCE' }
  return { sortBy: value.id.toUpperCase() as ListSortBy, sortOrder: value.desc ? 'desc' : 'asc' }
}
