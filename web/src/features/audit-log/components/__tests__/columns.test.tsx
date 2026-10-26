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
import i18n from '@/i18n'
import { renderWithProviders } from '@/test/utils'
import { screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { AuditEntry } from '@/api/audit/api'
import { getColumns } from '../columns'
import { AuditLogTable } from '../table'

function entry(id: string, event_type: string, success: boolean): AuditEntry {
  return {
    id,
    created_at: 0,
    event_type,
    actor: 'admin',
    target: id,
    ip: null,
    detail: null,
    success,
  }
}

function statusOf(target: string): HTMLElement {
  const row = screen
    .getByText(target, { selector: 'td *' })
    .closest('tr') as HTMLElement
  const cells = within(row).getAllByRole('cell')
  return cells[cells.length - 1]
}

describe('audit log status column', () => {
  it('shows the outcome of events that are also recorded on failure', () => {
    renderWithProviders(
      <AuditLogTable
        columns={getColumns(i18n.t)}
        data={[
          entry('login-ok', 'user.login', true),
          entry('enable-failed', 'mfa.enabled', false),
          entry('disable-ok', 'mfa.disabled', true),
          entry('reset', 'mfa.reset', true),
          entry('repair-failed', 'email.repaired', false),
        ]}
      />
    )

    expect(statusOf('login-ok')).toHaveTextContent('Success')
    expect(statusOf('enable-failed')).toHaveTextContent('Failed')
    expect(statusOf('disable-ok')).toHaveTextContent('Success')
    expect(statusOf('reset')).toHaveTextContent('—')
    expect(statusOf('repair-failed')).toHaveTextContent('Failed')
  })
})
