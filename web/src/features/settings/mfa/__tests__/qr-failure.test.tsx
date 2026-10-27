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
import { describe, expect, it, vi } from 'vitest'
import { EnrolmentCard } from '../enrolment-card'

vi.mock('qrcode', () => ({
  toString: () => Promise.reject(new Error('too long')),
}))

describe('EnrolmentCard', () => {
  it('falls back to the secret when the QR code cannot be generated', async () => {
    renderWithProviders(
      <EnrolmentCard
        enrolment={{
          secret: 'JBSWY3DPEHPK3PXP',
          otpauth_uri: 'otpauth://totp/x',
        }}
        onConfirmed={vi.fn()}
        onCancel={vi.fn()}
      />
    )

    expect(
      await screen.findByText(
        'The QR code could not be generated. Enter the secret manually.'
      )
    ).toBeInTheDocument()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(screen.getByText('JBSWY3DPEHPK3PXP')).toBeInTheDocument()
  })
})
