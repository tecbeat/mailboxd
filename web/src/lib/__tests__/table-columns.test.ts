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

import { HIDEABLE_COLUMNS, toColumnVisibility } from '../table-columns'

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

  it('keeps email, auth and enabled out of the optional accounts columns', () => {
    const ids = HIDEABLE_COLUMNS.accounts.map((column) => column.id as string)
    for (const mandatory of ['email', 'auth_type', 'enabled', 'actions']) {
      expect(ids).not.toContain(mandatory)
    }
  })
})
