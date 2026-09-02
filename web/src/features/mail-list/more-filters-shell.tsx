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
import { ListFilter } from 'lucide-react'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

interface MoreFiltersShellProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  activeCount: number
  triggerLabel: string
  title: string
  resetLabel: string
  applyLabel: string
  onReset: () => void
  onApply: () => void
  children: React.ReactNode
}

// Shared chrome for the "more filters" popovers: the trigger with its active
// count badge, the header with a reset action, and the apply button. The
// feature-specific filter fields are supplied as children; the feature owns
// its own local state, apply and reset logic (including closing the popover).
export function MoreFiltersShell({
  open,
  onOpenChange,
  activeCount,
  triggerLabel,
  title,
  resetLabel,
  applyLabel,
  onReset,
  onApply,
  children,
}: MoreFiltersShellProps) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className={cn(
            'h-6 gap-2 px-3 rounded-none border-l-0',
            activeCount > 0 && 'bg-primary/10 border-primary text-primary'
          )}
        >
          <ListFilter className="h-3.5 w-3.5" />
          <span className="text-xs">{triggerLabel}</span>
          {activeCount > 0 && (
            <Badge className="ml-1 h-4 px-1 text-[10px] bg-primary text-primary-foreground border-none rounded-xs">
              {activeCount}
            </Badge>
          )}
        </Button>
      </PopoverTrigger>

      <PopoverContent align="end" className="w-72 p-4 flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-medium">{title}</h4>
          {activeCount > 0 && (
            <Button
              variant="ghost"
              className="h-auto p-0 text-[10px] text-muted-foreground hover:text-destructive"
              onClick={onReset}
            >
              {resetLabel}
            </Button>
          )}
        </div>
        <Separator />

        {children}

        <Button size="sm" className="w-full h-8 text-xs mt-2" onClick={onApply}>
          {applyLabel}
        </Button>
      </PopoverContent>
    </Popover>
  )
}
