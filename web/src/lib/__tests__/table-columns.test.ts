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

import { describe, expect, it } from 'vitest'

import {
  TABLE_COLUMNS,
  defaultColumnSettings,
  fromColumnSettings,
  moveColumn,
  toColumnOrder,
  toColumnSettings,
  toColumnVisibility,
} from '../table-columns'

const DEFAULT_ORDER = [
  'id',
  'account_name',
  'email',
  'enabled',
  'auth_type',
  'account_type',
  'sync_interval_sec',
  'running_state',
  'created_by',
  'created_at',
  'updated_at',
]

describe('toColumnVisibility', () => {
  it('shows every column when nothing is hidden', () => {
    expect(toColumnVisibility('accounts', undefined)).toEqual({})
    expect(toColumnVisibility('accounts', [])).toEqual({})
  })

  it('hides the listed columns', () => {
    expect(toColumnVisibility('accounts', ['id', 'account_name'])).toEqual({
      id: false,
      account_name: false,
    })
  })

  it('keeps mandatory and unknown columns visible', () => {
    expect(toColumnVisibility('accounts', ['email', 'auth_type', 'enabled', 'nope'])).toEqual({})
  })

  it('never lets email, auth or enabled be hidden', () => {
    const hideable = TABLE_COLUMNS.accounts.filter((c) => c.hideable).map((c) => c.id as string)
    for (const mandatory of ['email', 'auth_type', 'enabled', 'actions']) {
      expect(hideable).not.toContain(mandatory)
    }
  })
})

describe('toColumnOrder', () => {
  it('uses the default order without a stored order', () => {
    expect(toColumnOrder('accounts', undefined)).toEqual(DEFAULT_ORDER)
    expect(toColumnOrder('accounts', [])).toEqual(DEFAULT_ORDER)
  })

  it('puts the stored columns first and the rest in default order', () => {
    expect(toColumnOrder('accounts', ['updated_at', 'email'])).toEqual([
      'updated_at',
      'email',
      ...DEFAULT_ORDER.filter((id) => id !== 'updated_at' && id !== 'email'),
    ])
  })

  it('drops unknown, repeated and fixed columns from a stored order', () => {
    expect(toColumnOrder('accounts', ['actions', 'nope', 'email', 'email'])).toEqual([
      'email',
      ...DEFAULT_ORDER.filter((id) => id !== 'email'),
    ])
  })
})

describe('column settings', () => {
  it('shows every column in default order by default', () => {
    expect(defaultColumnSettings('accounts')).toEqual(
      DEFAULT_ORDER.map((id) => ({ id, visible: true })),
    )
  })

  it('combines the stored order and hidden columns', () => {
    const settings = toColumnSettings('accounts', { order: ['created_at', 'id'], hidden: ['id', 'email'] })
    expect(settings.slice(0, 3)).toEqual([
      { id: 'created_at', visible: true },
      { id: 'id', visible: false },
      { id: 'account_name', visible: true },
    ])
    expect(settings.find((s) => s.id === 'email')).toEqual({ id: 'email', visible: true })
  })

  it('round-trips through the profile values', () => {
    const stored = { order: ['updated_at', 'email', 'id'], hidden: ['id', 'running_state'] }
    const values = fromColumnSettings('accounts', toColumnSettings('accounts', stored))
    expect(values.hidden.sort()).toEqual(['id', 'running_state'])
    expect(values.order.slice(0, 3)).toEqual(['updated_at', 'email', 'id'])
    expect(values.order).toHaveLength(DEFAULT_ORDER.length)
  })

  it('sends the default order as an empty list', () => {
    expect(fromColumnSettings('accounts', defaultColumnSettings('accounts'))).toEqual({
      order: [],
      hidden: [],
    })
  })

  it('never sends a mandatory column as hidden', () => {
    const settings = defaultColumnSettings('accounts').map((s) => ({ ...s, visible: false }))
    expect(fromColumnSettings('accounts', settings).hidden).not.toContain('email')
  })
})

describe('moveColumn', () => {
  const settings = ['a', 'b', 'c', 'd'].map((id) => ({ id, visible: id !== 'b' }))
  const ids = (list: { id: string }[]) => list.map((s) => s.id)

  it('moves a column down and up', () => {
    expect(ids(moveColumn(settings, 'a', 'c'))).toEqual(['b', 'c', 'a', 'd'])
    expect(ids(moveColumn(settings, 'd', 'b'))).toEqual(['a', 'd', 'b', 'c'])
  })

  it('keeps the visibility of the moved column', () => {
    expect(moveColumn(settings, 'b', 'd')[3]).toEqual({ id: 'b', visible: false })
  })

  it('returns the same list when nothing moves', () => {
    expect(moveColumn(settings, 'a', 'a')).toBe(settings)
    expect(moveColumn(settings, 'a', undefined)).toBe(settings)
    expect(moveColumn(settings, 'x', 'a')).toBe(settings)
  })
})
