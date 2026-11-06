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

import type { ColumnVisibilityState } from '@tanstack/react-table'

// Mirrors HIDEABLE_COLUMNS in crates/core/src/users/preferences.rs, which
// rejects any other column.
export const HIDEABLE_COLUMNS = {
  accounts: [
    { id: 'id', labelKey: 'accounts.id' },
    { id: 'account_name', labelKey: 'accounts.name' },
    { id: 'account_type', labelKey: 'accounts.type' },
    { id: 'sync_interval_sec', labelKey: 'accounts.scheduleMode' },
    { id: 'running_state', labelKey: 'accounts.state' },
    { id: 'created_by', labelKey: 'accounts.owner' },
    { id: 'created_at', labelKey: 'accounts.createdAt' },
    { id: 'updated_at', labelKey: 'accounts.updatedAt' },
  ],
} as const

export type HideableTable = keyof typeof HIDEABLE_COLUMNS

/** The table visibility state for the hidden column ids of a profile. */
export function toColumnVisibility(
  table: HideableTable,
  hidden: readonly string[] | undefined,
): ColumnVisibilityState {
  const allowed = new Set<string>(HIDEABLE_COLUMNS[table].map((column) => column.id))
  return Object.fromEntries(
    (hidden ?? []).filter((id) => allowed.has(id)).map((id) => [id, false]),
  )
}
