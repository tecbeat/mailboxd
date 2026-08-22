//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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

import {
  columnFacetingFeature,
  columnFilteringFeature,
  columnSizingFeature,
  columnVisibilityFeature,
  createFacetedRowModel,
  createFacetedUniqueValues,
  createFilteredRowModel,
  createPaginatedRowModel,
  createSortedRowModel,
  filterFns,
  globalFilteringFeature,
  rowPaginationFeature,
  rowSelectionFeature,
  rowSortingFeature,
  sortFns,
  tableFeatures,
} from '@tanstack/react-table'

/**
 * Per-column `meta` shape shared by every app table.
 *
 * In v9 this is declared through the `columnMeta` slot of {@link tableFeatures}
 * instead of the global `declare module '@tanstack/react-table'` augmentation
 * that TanStack Table v8 required.
 */
type AppColumnMeta = { className: string }

/**
 * Feature set for the CRUD data tables (accounts, users, roles, API tokens,
 * OAuth2 clients, proxies): sorting, column + global filtering, pagination,
 * faceting, column visibility and row selection.
 */
export const dataTableFeatures = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns,
  columnFilteringFeature,
  filteredRowModel: createFilteredRowModel(),
  filterFns,
  globalFilteringFeature,
  rowPaginationFeature,
  paginatedRowModel: createPaginatedRowModel(),
  columnVisibilityFeature,
  columnSizingFeature,
  rowSelectionFeature,
  columnFacetingFeature,
  facetedRowModel: createFacetedRowModel(),
  facetedUniqueValues: createFacetedUniqueValues(),
  columnMeta: {} as AppColumnMeta,
})

export type DataTableFeatures = typeof dataTableFeatures

/**
 * Feature set for the virtualized mail-list tables (search, attachment): the
 * same features as {@link dataTableFeatures} but without pagination, because
 * these tables render the full filtered row model inside a scroll area.
 */
export const mailTableFeatures = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns,
  columnFilteringFeature,
  filteredRowModel: createFilteredRowModel(),
  filterFns,
  globalFilteringFeature,
  columnVisibilityFeature,
  columnSizingFeature,
  rowSelectionFeature,
  columnFacetingFeature,
  facetedRowModel: createFacetedRowModel(),
  facetedUniqueValues: createFacetedUniqueValues(),
  columnMeta: {} as AppColumnMeta,
})

export type MailTableFeatures = typeof mailTableFeatures
