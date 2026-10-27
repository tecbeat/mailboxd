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

import { useEffect } from 'react'
import { Sparkles } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'
import { useMailListConfig } from '@/features/mail-list/config'
import { FilterLabel } from '@/features/mail-list/filter-bar'
import { filterSegment } from '@/features/mail-list/filter-bar-styles'
import { DEFAULT_SORTING } from '@/features/mail-list/sorting'

/**
 * Filter-bar toggle for relevance sorting. Only available while one of
 * `textFields` is set, because without a text query there is no score; the
 * list falls back to date order once the text is cleared.
 */
export function RelevanceSortButton({ textFields }: { textFields: string[] }) {
  const { t } = useTranslation()
  const { useListContext } = useMailListConfig()
  const { filter, sorting, setSorting } = useListContext()

  const hasText = textFields.some((field) => Boolean(filter[field]))
  const byRelevance = sorting.length === 0

  useEffect(() => {
    if (byRelevance && !hasText) setSorting(DEFAULT_SORTING)
  }, [byRelevance, hasText, setSorting])

  const active = byRelevance && hasText
  const title = hasText ? t('search_sort.relevance_tooltip') : t('search_sort.relevance_needs_text')

  return (
    <Button
      variant='ghost'
      className={filterSegment(active)}
      aria-pressed={active}
      disabled={!hasText}
      title={title}
      onClick={() => setSorting(active ? DEFAULT_SORTING : [])}
    >
      <Sparkles className='h-4 w-4' />
      <FilterLabel>{t('search_sort.relevance')}</FilterLabel>
    </Button>
  )
}
