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

import * as React from 'react'
import { cn } from '@/lib/utils'

// Whether the current bar is "dense" (many segments). Dense bars only expand
// their labels at a wider container width so labelled segments never wrap into
// a second row before there is genuinely room for them.
const FilterBarDensityContext = React.createContext(false)

// One unified filter bar. Search input and every filter live inside a single
// bordered container so the toolbar reads as one component rather than a search
// box plus a loose row of chips. The bar is a container-query context
// (`@container/fbar`): each segment keeps its label while the bar is wide and
// collapses to icon-only via `FilterLabel` once space runs out — no horizontal
// scrolling. When the bar can no longer fit one row it wraps.
export function FilterBar({
  className,
  dense = false,
  children,
}: {
  className?: string
  dense?: boolean
  children: React.ReactNode
}) {
  return (
    <FilterBarDensityContext.Provider value={dense}>
      <div
        className={cn(
          '@container/fbar flex min-h-9 flex-wrap items-stretch',
          'divide-x divide-border rounded-md border bg-background',
          'transition-colors focus-within:border-primary/50 focus-within:ring-1 focus-within:ring-primary/30',
          // Keep the rounded corners visible even when the first/last segment
          // paints an active background.
          '[&>*:first-child]:rounded-l-md [&>*:last-child]:rounded-r-md',
          className
        )}
      >
        {children}
      </div>
    </FilterBarDensityContext.Provider>
  )
}

// A segment label that is shown while the bar is wide and hidden (leaving the
// icon only) once the bar becomes narrow. Dense bars (many segments) only
// reveal labels at a wider container width so labelled segments do not wrap.
export function FilterLabel({
  className,
  children,
}: {
  className?: string
  children: React.ReactNode
}) {
  const dense = React.useContext(FilterBarDensityContext)
  return (
    <span
      className={cn(
        dense ? 'hidden @6xl/fbar:inline' : 'hidden @4xl/fbar:inline',
        className
      )}
    >
      {children}
    </span>
  )
}

// Small numeric badge shown next to a segment icon. Stays visible even when the
// label is collapsed so an active filter is always discoverable.
export function FilterCount({ count }: { count?: number }) {
  if (!count) return null
  return (
    <span className="inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] font-semibold leading-none text-primary-foreground">
      {count}
    </span>
  )
}

// Dot indicator for value-based filters (where there is no meaningful count).
// Marks the segment as active when its label is collapsed.
export function ActiveDot({ active }: { active?: boolean }) {
  if (!active) return null
  return <span className="h-1.5 w-1.5 rounded-full bg-primary" />
}
