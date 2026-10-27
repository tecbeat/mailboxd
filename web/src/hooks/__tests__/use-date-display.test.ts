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

import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { DATE_DISPLAY_STORAGE_KEY, useDateDisplay } from '../use-date-display'

describe('useDateDisplay', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => vi.restoreAllMocks())

  it('defaults to relative and persists a change', () => {
    const { result } = renderHook(() => useDateDisplay())
    expect(result.current.dateDisplay).toBe('relative')

    act(() => result.current.setDateDisplay('absolute'))

    expect(result.current.dateDisplay).toBe('absolute')
    expect(localStorage.getItem(DATE_DISPLAY_STORAGE_KEY)).toBe('absolute')
  })

  it('ignores an unknown stored value', () => {
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'sideways')
    const { result } = renderHook(() => useDateDisplay())
    expect(result.current.dateDisplay).toBe('relative')
  })

  it('updates every user of the hook at once', () => {
    const first = renderHook(() => useDateDisplay())
    const second = renderHook(() => useDateDisplay())

    act(() => first.result.current.setDateDisplay('absolute'))

    expect(second.result.current.dateDisplay).toBe('absolute')
  })

  it('follows a change made in another tab', () => {
    const { result } = renderHook(() => useDateDisplay())

    // Another tab writes the key; this tab only sees the storage event.
    localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, 'absolute')
    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: 'unrelated' }))
    })
    expect(result.current.dateDisplay).toBe('relative')

    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: DATE_DISPLAY_STORAGE_KEY }))
    })
    expect(result.current.dateDisplay).toBe('absolute')
  })

  it('keeps the choice in memory when localStorage is unavailable', () => {
    const blocked = () => {
      throw new DOMException('blocked', 'SecurityError')
    }
    // Spy on the prototype: jsdom stores properties set on localStorage itself.
    const getItem = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(blocked)
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(blocked)
    const { result } = renderHook(() => useDateDisplay())

    act(() => result.current.setDateDisplay('absolute'))
    expect(result.current.dateDisplay).toBe('absolute')

    expect(getItem).toHaveBeenCalled()
    expect(setItem).toHaveBeenCalled()

    act(() => result.current.setDateDisplay('relative'))
    expect(result.current.dateDisplay).toBe('relative')

    vi.restoreAllMocks()
    expect(localStorage.length).toBe(0)
  })
})
