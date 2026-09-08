//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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

/**
 * localStorage key under which a table/list view persists its chosen page size.
 */
const pageSizeStorageKey = (storageKey: string): string =>
  `mailboxd_${storageKey}_page_size`

/**
 * Read the persisted page size for a view. Returns `fallback` when nothing is
 * stored, the stored value is not a positive finite number, or `localStorage`
 * is unavailable (SSR, private mode).
 */
export function getPersistedPageSize(storageKey: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(pageSizeStorageKey(storageKey))
    if (raw === null) return fallback
    const parsed = Number.parseInt(raw, 10)
    return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
  } catch {
    return fallback
  }
}

/**
 * Persist the chosen page size for a view. No-op when `localStorage` is
 * unavailable (SSR, private mode).
 */
export function setPersistedPageSize(storageKey: string, size: number): void {
  try {
    localStorage.setItem(pageSizeStorageKey(storageKey), String(size))
  } catch {
    // localStorage unavailable; persistence is best-effort.
  }
}
