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
import { AtSign, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { cn } from '@/lib/utils'
import { useMailListConfig } from '@/features/mail-list/config'
import { FilterLabel, FilterCount } from '@/features/mail-list/filter-bar'
import { filterSegment } from '@/features/mail-list/filter-bar-styles'

export function AccountPopover() {
  const { t } = useTranslation()
  const { useListContext } = useMailListConfig()
  const { filter, setFilter } = useListContext()
  const [search, setSearch] = React.useState('')
  const { minimalList = [] } = useMinimalAccountList()

  const selectedIds: number[] = React.useMemo(() => filter.account_ids ?? [], [filter.account_ids])

  const toggleAccount = (id: number) => {
    setFilter(prev => {
      const next = { ...prev }
      const set = new Set<number>(next.account_ids ?? [])

      if (set.has(id)) {
        set.delete(id)
      } else {
        set.add(id)
      }

      if (set.size === 0) {
        delete next.account_ids
        delete next.mailbox_ids
      } else {
        next.account_ids = Array.from(set).sort()
        delete next.mailbox_ids
      }

      return next
    })
  }

  const clearAccounts = () => {
    setFilter(prev => {
      const next = { ...prev }
      delete next.account_ids
      delete next.mailbox_ids
      return next
    })
  }

  const filtered = React.useMemo(() => {
    const q = search.toLowerCase()

    return minimalList
      .filter(a =>
        !q ||
        a.email.toLowerCase().includes(q) ||
        a.name?.toLowerCase().includes(q) ||
        String(a.id).includes(q)
      )
      .sort((a, b) => {
        const aSel = selectedIds.includes(a.id)
        const bSel = selectedIds.includes(b.id)

        if (aSel && !bSel) return -1
        if (!aSel && bSel) return 1
        return a.id - b.id
      })
  }, [minimalList, search, selectedIds])

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="ghost"
          className={filterSegment(selectedIds.length > 0)}
          title={t('search_accounts.label')}
        >
          <AtSign className="h-4 w-4" />
          <FilterLabel>{t('search_accounts.label')}</FilterLabel>
          <FilterCount count={selectedIds.length} />
        </Button>
      </PopoverTrigger>

      <PopoverContent align="start" className="w-96 p-1">
        <div className="p-1 pb-2">
          <Input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('search_accounts.search_placeholder')}
            className="h-8 text-sm"
          />
        </div>
        {!search && selectedIds.length > 0 && (
          <div className="p-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={clearAccounts}
              className="flex h-8 w-full items-center justify-start gap-2 px-2 text-xs font-medium text-destructive hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
              <div className="flex h-4 w-4 items-center justify-center">
                <X className="h-3.5 w-3.5" />
              </div>
              <span className="flex-1 text-left">
                {t('search_accounts.clear_accounts')}
              </span>
              <span className="text-[10px] opacity-60 font-mono">
                ({selectedIds.length})
              </span>
            </Button>
            <div className="my-1 h-px bg-border/60" />
          </div>
        )}
        <ScrollArea className="h-96 p-1">
          {filtered.length === 0 ? (
            <p className="px-3 py-2 text-xs text-muted-foreground">
              {t('search_accounts.no_accounts_found')}
            </p>
          ) : (
            filtered.map(account => {
              const checked = selectedIds.includes(account.id)
              const id = `account-${account.id}`

              return (
                <div
                  key={account.id}
                  onClick={() => toggleAccount(account.id)}
                  className={cn(
                    'flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer',
                    'hover:bg-accent transition-colors'
                  )}
                >
                  <Checkbox
                    id={id}
                    checked={checked}
                    onCheckedChange={() =>
                      toggleAccount(account.id)
                    }
                    onClick={e => e.stopPropagation()}
                  />

                  <Label
                    htmlFor={id}
                    className="flex-1 truncate text-xs cursor-pointer"
                  >
                    <div className="flex items-center gap-2">
                      {account.name ? (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span className="truncate">
                              {account.name}
                            </span>
                          </TooltipTrigger>
                          <TooltipContent side="top">
                            {account.email}
                          </TooltipContent>
                        </Tooltip>
                      ) : (
                        <span className="truncate">
                          {account.email}
                        </span>
                      )}
                      <span className="text-[10px] text-muted-foreground">
                        #{account.id}
                      </span>
                    </div>
                  </Label>
                </div>
              )
            })
          )}
        </ScrollArea>
      </PopoverContent>
    </Popover>
  )
}