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

import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { createFileRoute, Outlet } from '@tanstack/react-router'
import { cn } from '@/lib/utils'
import { SidebarProvider } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/layout/app-sidebar'
import { useQueryClient } from '@tanstack/react-query'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useTheme, Theme } from '@/context/theme-context'
import { applyProfileDatePatterns } from '@/hooks/use-date-format'
import { update_user } from '@/api/users/api'
import { toast } from '@/hooks/use-toast'

export const Route = createFileRoute('/_authenticated')({
  component: RouteComponent,
})

function RouteComponent() {
  const defaultOpen = localStorage.getItem('sidebar_state') === 'true';
  const { user } = useCurrentUser()
  const { setTheme } = useTheme()
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const prefsApplied = useRef(false)

  // On first load of the authenticated shell, initialise theme, language
  // and date patterns from the user profile (authoritative), overriding the localStorage
  // fallback the ThemeProvider used to avoid a flash. Runs once, so it never
  // clobbers an in-session change made via the header switch or the
  // Appearance form (both of which persist to the same profile).
  useEffect(() => {
    if (prefsApplied.current || !user) return
    prefsApplied.current = true
    if (user.theme) setTheme(user.theme as Theme)
    if (user.language && user.language !== i18n.language) {
      i18n.changeLanguage(user.language)
    }
    applyProfileDatePatterns(user, async (legacy) => {
      await update_user(user.id, { date_pattern: legacy.date, date_time_pattern: legacy.dateTime })
      await queryClient.invalidateQueries({ queryKey: ['current-user'] })
    }).catch(() => {
      toast({ variant: 'destructive', title: t('settings.appearance.date_pattern.migration_failed') })
    })
  }, [user, setTheme, t, i18n, queryClient])
  return (
    <SidebarProvider defaultOpen={defaultOpen}>
      <AppSidebar />
      <div
        id='content'
        className={cn(
          'max-w-full w-full ml-auto',
          'peer-data-[state=collapsed]:w-[calc(100%-var(--sidebar-width-icon)-1rem)]',
          'peer-data-[state=expanded]:w-[calc(100%-var(--sidebar-width))]',
          'transition-[width] ease-linear duration-200',
          // Sole vertical scroll region of the app shell: the sidebar and the
          // fixed header stay pinned while this column scrolls. `relative` makes
          // this the containing block for absolutely-positioned descendants
          // (row accents, sr-only spans) so they cannot leak past the scroll
          // clip and give the document a phantom, focus-scrollable height.
          'relative h-svh flex flex-col overflow-y-auto overflow-x-hidden scrollbar-thin',
          // Freeze background scrolling while a Radix dialog locks the body.
          'group-data-[scroll-locked=1]/body:overflow-hidden'
        )}
      >
        <Outlet />
      </div>
    </SidebarProvider>
  )
}
