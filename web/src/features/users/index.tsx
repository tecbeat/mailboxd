//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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


import { Outlet } from '@tanstack/react-router'
import { Main } from '@/components/layout/main'
import { SidebarNav } from '@/components/layout/sidebar-nav'
import { Users, ShieldCheck, Key } from "lucide-react";

import { FixedHeader } from '@/components/layout/fixed-header'
import { useTranslation } from 'react-i18next'

export default function UsersAndTokens() {
  const { t } = useTranslation()

  const sidebarNavItems = [
    {
      title: t('users.nav.users'),
      icon: <Users size={18} />,
      href: '/users',
    },
    {
      title: t('users.nav.roles'),
      icon: <ShieldCheck size={18} />,
      href: '/users/roles',
    },
    {
      title: t('users.nav.api_tokens'),
      icon: <Key size={18} />,
      href: '/users/api-tokens',
    },
  ]

  return (
    <>
      <FixedHeader />
      <Main fixed>
        <div className='flex min-h-0 flex-1 flex-col space-y-2 overflow-hidden lg:flex-row lg:space-x-12 lg:space-y-0'>
          {/* Fixed-width rail so the nav never changes width between sub-pages. */}
          <aside className='shrink-0 lg:w-56'>
            <SidebarNav items={sidebarNavItems} />
          </aside>
          {/*
            min-w-0 stops a wide table from widening the row (and the rail).
            scrollbar-gutter:stable always reserves the vertical scrollbar's
            space, so the content width stays constant between sub-pages that
            scroll (e.g. Users) and ones that don't, instead of jumping
            sideways by the scrollbar width when switching. Mirrors the
            Settings section shell so both headers align identically.
          */}
          <div className='flex min-h-0 w-full min-w-0 flex-1 flex-col overflow-y-auto scrollbar-thin p-1 pr-4 [scrollbar-gutter:stable]'>
            <Outlet />
          </div>
        </div>
      </Main>
    </>
  )
}


