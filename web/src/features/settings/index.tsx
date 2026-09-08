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
import { KeyRound, Palette, Settings as SettingsIcon, ShieldCheck, UserCog, Waypoints } from 'lucide-react'
import { FixedHeader } from '@/components/layout/fixed-header'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useTranslation } from 'react-i18next'

export default function Settings() {
  const { t } = useTranslation()
  const { canGlobal } = useCurrentUser()


  const sidebarNavItems = [
    {
      title: t('settings.sidebar.profile'),
      href: '/settings/profile',
      icon: <UserCog size={18} />,
    },
    {
      title: t('settings.sidebar.access'),
      href: '/settings/access',
      icon: <ShieldCheck size={18} />
    },
    {
      title: t('settings.appearance.title'),
      href: '/settings/appearance',
      icon: <Palette size={18} />,
    },
    {
      title: t('settings.sidebar.apiTokens'),
      href: '/settings/api-tokens',
      icon: <KeyRound size={18} />,
    },
    {
      title: t('settings.sidebar.proxy'),
      icon: <Waypoints size={18} />,
      href: '/settings/proxy',
    },
    {
      title: t('settings.sidebar.configurations'),
      icon: <SettingsIcon size={18} />,
      href: '/settings/configurations',
      visible: canGlobal('system:root'),
    },
  ].filter(item => item.visible !== false)

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
            No right padding or reserved scrollbar gutter here: the content's
            right edge must line up with the primary-nav pages (Accounts,
            OAuth2, ...), which render flush against Main's padding. Sub-pages
            with add buttons (API Tokens, Proxy) scroll internally, so this
            container never shows a scrollbar on them. The only sub-page that
            overflows this container (Configurations) has no header actions, so
            its scrollbar appearing at the flush edge is not noticeable.
          */}
          <div className='flex min-h-0 w-full min-w-0 flex-1 flex-col overflow-y-auto scrollbar-thin'>
            <Outlet />
          </div>
        </div>
      </Main>
    </>
  )
}


