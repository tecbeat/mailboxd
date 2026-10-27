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
import { renderWithProviders } from '@/test/utils'
import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { AccountModel } from '@/api/account/api'
import { AccountDetailDrawer } from '../account-detail'

const account = (over: Partial<AccountModel> = {}): AccountModel => ({
  id: 5,
  account_type: 'NoSync',
  enabled: true,
  email: 'archive@example.com',
  download_folders: [],
  created_by: 1,
  created_user_name: 'admin',
  created_user_email: 'admin@example.com',
  created_at: 0,
  updated_at: 0,
  use_dangerous: false,
  ...over,
})

function retentionShown(over: Partial<AccountModel>) {
  renderWithProviders(
    <AccountDetailDrawer
      open
      onOpenChange={() => {}}
      currentRow={account(over)}
    />
  )
  return screen.getByText('Retention period (days):').nextElementSibling
}

describe('AccountDetailDrawer retention', () => {
  it('shows the retention period in days', () => {
    expect(retentionShown({ retention_days: 30 })).toHaveTextContent(/^30$/)
  })

  it.each([undefined, null, 0])(
    'shows that mail is kept forever for %s',
    (retention_days) => {
      expect(retentionShown({ retention_days })).toHaveTextContent(
        'Keep forever'
      )
    }
  )
})
