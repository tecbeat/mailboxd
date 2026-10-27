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

import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { renderWithProviders } from '@/test/utils'

const { getNotifications } = vi.hoisted(() => ({ getNotifications: vi.fn() }))

vi.mock('@/api/system/api', () => ({ get_notifications: getNotifications }))
vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, children, className }: { to: string; children: ReactNode; className?: string }) => (
    <a href={to} className={className}>
      {children}
    </a>
  ),
}))

import { NotificationPopover } from '../notification'

const RELEASE = {
  tag_name: 'v9.9.9',
  published_at: '2026-10-01T00:00:00Z',
  body: 'Release notes',
  html_url: 'https://example.com/releases/v9.9.9',
}

function notifications(defaultAdminPassword: boolean, newRelease = false) {
  return {
    release: newRelease
      ? { latest: RELEASE, is_newer: true, error_message: null }
      : { latest: null, is_newer: false, error_message: null },
    default_admin_password: defaultAdminPassword,
  }
}

describe('NotificationPopover', () => {
  beforeEach(() => getNotifications.mockReset())

  it('reminds about the default admin password and links to the profile', async () => {
    getNotifications.mockResolvedValue(notifications(true))
    renderWithProviders(<NotificationPopover />)

    const bell = await screen.findByRole('button', { name: '1' })
    await userEvent.click(bell)

    expect(await screen.findByText('Default admin password in use')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Change password' })).toHaveAttribute(
      'href',
      '/settings/profile',
    )
  })

  it('shows nothing once the password was changed', async () => {
    getNotifications.mockResolvedValue(notifications(false))
    renderWithProviders(<NotificationPopover />)

    const bell = await screen.findByRole('button')
    await vi.waitFor(() => expect(bell).toBeEnabled())
    await userEvent.click(bell)

    expect(await screen.findByText('No new notifications')).toBeInTheDocument()
    expect(screen.queryByText('Default admin password in use')).not.toBeInTheDocument()
  })

  it('lists the reminder next to a new release', async () => {
    getNotifications.mockResolvedValue(notifications(true, true))
    renderWithProviders(<NotificationPopover />)

    await userEvent.click(await screen.findByRole('button', { name: '2' }))

    expect(await screen.findByText('Default admin password in use')).toBeInTheDocument()
    expect(screen.getByText('v9.9.9')).toBeInTheDocument()
  })
})
