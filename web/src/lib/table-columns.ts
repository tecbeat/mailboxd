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

import type { ColumnOrderState, ColumnVisibilityState } from '@tanstack/react-table'

export interface TableColumn {
  id: string
  labelKey: string
  /** Columns that cannot be hidden are always shown, but can still be moved. */
  hideable: boolean
}

// The columns a user can move, in their default order. Mirrors
// ORDERABLE_COLUMNS and HIDEABLE_COLUMNS in crates/core/src/users/preferences.rs,
// which reject any other column. The actions column always stays last.
export const TABLE_COLUMNS = {
  accounts: [
    { id: 'id', labelKey: 'accounts.id', hideable: true },
    { id: 'account_name', labelKey: 'accounts.name', hideable: true },
    { id: 'email', labelKey: 'accounts.email', hideable: false },
    { id: 'enabled', labelKey: 'accounts.enabled', hideable: false },
    { id: 'auth_type', labelKey: 'accounts.auth', hideable: false },
    { id: 'account_type', labelKey: 'accounts.type', hideable: true },
    { id: 'sync_interval_sec', labelKey: 'accounts.scheduleMode', hideable: true },
    { id: 'running_state', labelKey: 'accounts.state', hideable: true },
    { id: 'created_by', labelKey: 'accounts.owner', hideable: true },
    { id: 'created_at', labelKey: 'accounts.createdAt', hideable: true },
    { id: 'updated_at', labelKey: 'accounts.updatedAt', hideable: true },
  ],
} as const satisfies Record<string, readonly TableColumn[]>

export type ConfigurableTable = keyof typeof TABLE_COLUMNS

/** The table visibility state for the hidden column ids of a profile. */
export function toColumnVisibility(
  table: ConfigurableTable,
  hidden: readonly string[] | undefined,
): ColumnVisibilityState {
  const allowed = new Set<string>(
    TABLE_COLUMNS[table].filter((column) => column.hideable).map((column) => column.id),
  )
  return Object.fromEntries(
    (hidden ?? []).filter((id) => allowed.has(id)).map((id) => [id, false]),
  )
}

/**
 * Every movable column id of `table`, in the order of the profile. Unknown
 * and repeated ids are dropped; columns the profile does not list follow in
 * their default order.
 */
export function toColumnOrder(
  table: ConfigurableTable,
  stored: readonly string[] | undefined,
): ColumnOrderState {
  const defaults: string[] = TABLE_COLUMNS[table].map((column) => column.id)
  const known = new Set(defaults)
  const order = [...new Set(stored ?? [])].filter((id) => known.has(id))
  return [...order, ...defaults.filter((id) => !order.includes(id))]
}

/** One row of the column settings: a movable column and whether it is shown. */
export interface ColumnSetting {
  id: string
  visible: boolean
}

/** The column settings of `table` for the stored profile values. */
export function toColumnSettings(
  table: ConfigurableTable,
  stored: { order?: readonly string[]; hidden?: readonly string[] },
): ColumnSetting[] {
  const hidden = toColumnVisibility(table, stored.hidden)
  return toColumnOrder(table, stored.order).map((id) => ({ id, visible: hidden[id] !== false }))
}

/** The default column settings of `table`: every column shown, default order. */
export function defaultColumnSettings(table: ConfigurableTable): ColumnSetting[] {
  return toColumnSettings(table, {})
}

/**
 * The profile values for `settings`. An unchanged default order is sent as
 * an empty list, so columns added later keep their default position.
 */
export function fromColumnSettings(
  table: ConfigurableTable,
  settings: readonly ColumnSetting[],
): { order: string[]; hidden: string[] } {
  const hideable = new Set<string>(
    TABLE_COLUMNS[table].filter((column) => column.hideable).map((column) => column.id),
  )
  const order = settings.map((setting) => setting.id)
  const isDefault = order.join('\n') === toColumnOrder(table, []).join('\n')
  return {
    order: isDefault ? [] : order,
    hidden: settings.filter((s) => !s.visible && hideable.has(s.id)).map((s) => s.id),
  }
}

/** Move the column `active` to the position of the column `over`. */
export function moveColumn(
  settings: ColumnSetting[],
  active: string,
  over: string | undefined,
): ColumnSetting[] {
  const from = settings.findIndex((setting) => setting.id === active)
  const to = settings.findIndex((setting) => setting.id === over)
  if (from < 0 || to < 0 || from === to) return settings
  const next = [...settings]
  const [moved] = next.splice(from, 1)
  next.splice(to, 0, moved)
  return next
}
