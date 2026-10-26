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
import type { ReactNode } from 'react'
import { renderWithProviders } from '@/test/utils'
import { screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import Settings from '../index'

vi.mock('@tanstack/react-router', () => ({ Outlet: () => null }))
vi.mock('@/hooks/use-current-user', () => ({
  useCurrentUser: () => ({ canGlobal: () => false }),
}))
vi.mock('@/components/layout/fixed-header', () => ({ FixedHeader: () => null }))
vi.mock('@/components/layout/main', () => ({
  Main: ({ children }: { children: ReactNode }) => <main>{children}</main>,
}))
vi.mock('@/components/layout/sidebar-nav', () => ({
  SidebarNav: ({ items }: { items: { title: string; href: string }[] }) => (
    <nav>
      {items.map((item) => (
        <a key={item.href} href={item.href}>
          {item.title}
        </a>
      ))}
    </nav>
  ),
}))

describe('Settings navigation', () => {
  it('links to the two-factor settings for every user', () => {
    renderWithProviders(<Settings />)

    expect(
      screen.getByRole('link', { name: 'Two-Factor Authentication' })
    ).toHaveAttribute('href', '/settings/mfa')
  })
})
