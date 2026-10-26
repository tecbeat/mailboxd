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

import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useDateDisplay } from '@/hooks/use-date-display'
import { formatDateTime, formatRelativeTime } from '@/lib/utils'

/** A list date in the chosen display mode; the tooltip shows the other one. */
export function DateCell({ date }: { date: number | string | Date }) {
  const { dateDisplay } = useDateDisplay()
  const absolute = formatDateTime(date)
  const relative = formatRelativeTime(date)
  const [shown, hint] = dateDisplay === 'absolute' ? [absolute, relative] : [relative, absolute]
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className='text-xs whitespace-nowrap'>{shown}</span>
      </TooltipTrigger>
      <TooltipContent>{hint}</TooltipContent>
    </Tooltip>
  )
}
