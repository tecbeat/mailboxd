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

import { DATE_FORMAT_STORAGE_KEY, readDateFormat, useDateFormat } from '../use-date-format'

describe('useDateFormat', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => vi.restoreAllMocks())

  it('defaults to the locale format and persists a change', () => {
    const { result } = renderHook(() => useDateFormat())
    expect(result.current.dateFormat).toBe('locale')

    act(() => result.current.setDateFormat('iso'))

    expect(result.current.dateFormat).toBe('iso')
    expect(readDateFormat()).toBe('iso')
    expect(localStorage.getItem(DATE_FORMAT_STORAGE_KEY)).toBe('iso')
  })

  it('ignores an unknown stored value', () => {
    localStorage.setItem(DATE_FORMAT_STORAGE_KEY, 'roman')
    const { result } = renderHook(() => useDateFormat())
    expect(result.current.dateFormat).toBe('locale')
  })

  it('updates every user of the hook at once', () => {
    const first = renderHook(() => useDateFormat())
    const second = renderHook(() => useDateFormat())

    act(() => first.result.current.setDateFormat('iso'))

    expect(second.result.current.dateFormat).toBe('iso')
  })

  it('follows a change made in another tab', () => {
    const { result } = renderHook(() => useDateFormat())

    localStorage.setItem(DATE_FORMAT_STORAGE_KEY, 'iso')
    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: 'unrelated' }))
    })
    expect(result.current.dateFormat).toBe('locale')

    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: DATE_FORMAT_STORAGE_KEY }))
    })
    expect(result.current.dateFormat).toBe('iso')
  })

  it('keeps the choice in memory when localStorage is unavailable', () => {
    const blocked = () => {
      throw new DOMException('blocked', 'SecurityError')
    }
    // Spy on the prototype: jsdom stores properties set on localStorage itself.
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(blocked)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(blocked)
    const { result } = renderHook(() => useDateFormat())

    act(() => result.current.setDateFormat('iso'))
    expect(result.current.dateFormat).toBe('iso')

    act(() => result.current.setDateFormat('locale'))
    expect(result.current.dateFormat).toBe('locale')

    vi.restoreAllMocks()
    expect(localStorage.length).toBe(0)
  })
})
