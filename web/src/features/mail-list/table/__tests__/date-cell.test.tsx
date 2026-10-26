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

import { act, render, screen } from '@/test/test-utils'
import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { TooltipProvider } from '@/components/ui/tooltip'
import { DATE_DISPLAY_STORAGE_KEY, useDateDisplay } from '@/hooks/use-date-display'
import { formatDateTime, formatRelativeTime } from '@/lib/utils'

import { DateCell } from '../date-cell'

const DATE = Date.UTC(2024, 0, 15, 9, 30)

function renderCell() {
  return render(
    <TooltipProvider>
      <DateCell date={DATE} />
    </TooltipProvider>,
  )
}

describe('DateCell', () => {
  afterEach(() => localStorage.clear())

  it('shows the relative date by default', () => {
    renderCell()
    expect(screen.getByText(formatRelativeTime(DATE))).toBeInTheDocument()
  })

  it('shows the absolute date once chosen, and keeps the choice', () => {
    const { result } = renderHook(() => useDateDisplay())
    renderCell()

    act(() => result.current.setDateDisplay('absolute'))

    expect(screen.getByText(formatDateTime(DATE))).toBeInTheDocument()
    expect(localStorage.getItem(DATE_DISPLAY_STORAGE_KEY)).toBe('absolute')
  })

  it('reads a stored choice and ignores unknown values', () => {
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'absolute')
    const { unmount } = renderCell()
    expect(screen.getByText(formatDateTime(DATE))).toBeInTheDocument()
    unmount()

    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'bogus')
    renderCell()
    expect(screen.getByText(formatRelativeTime(DATE))).toBeInTheDocument()
  })
})
