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

import { cn } from '@/lib/utils'

// Shared styling for every trigger that sits inside the filter bar (popover
// buttons, dropdown triggers, the reset button). Segments are borderless and
// share the bar's height; the bar's `divide-x` supplies the separators between
// them.
export function filterSegment(active?: boolean) {
  return cn(
    'inline-flex h-9 shrink-0 items-center gap-1.5 whitespace-nowrap',
    'rounded-none border-0 bg-transparent px-2.5 text-xs font-normal text-foreground shadow-none',
    'hover:bg-accent focus-visible:ring-0 focus-visible:ring-offset-0',
    'disabled:pointer-events-none disabled:opacity-50',
    // Once the bar is narrow enough to wrap, let the (now icon-only) segments
    // grow evenly and centre their icons so each wrapped row fills the bar's
    // full width instead of leaving empty space on one side.
    '@max-xl/fbar:flex-1 @max-xl/fbar:justify-center',
    active && 'bg-primary/10 text-primary hover:bg-primary/15'
  )
}
