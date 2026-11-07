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

import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  LEGACY_DATE_PATTERN_STORAGE_KEYS as LEGACY,
  applyProfileDatePatterns,
  readDateFnsPattern,
  setDatePatterns,
  useDateFormat,
} from '../use-date-format'

describe('useDateFormat', () => {
  afterEach(() => {
    localStorage.clear()
    setDatePatterns({ date: '', dateTime: '' })
    vi.restoreAllMocks()
  })

  it('defaults to the language format and applies a change', () => {
    const { result } = renderHook(() => useDateFormat())
    expect(result.current.datePattern).toBe('')
    expect(result.current.dateTimePattern).toBe('')
    expect(readDateFnsPattern('date')).toBeNull()

    act(() => result.current.setDatePatterns({ date: ' MM.YY ', dateTime: 'DD.MM.YYYY HH:mm' }))

    expect(result.current.datePattern).toBe('MM.YY')
    expect(result.current.dateTimePattern).toBe('DD.MM.YYYY HH:mm')
    expect(readDateFnsPattern('date')).toBe('MM.yy')
    expect(readDateFnsPattern('dateTime')).toBe('dd.MM.yyyy HH:mm')
    expect(localStorage.length).toBe(0)
  })

  it('falls back to the language format for an invalid pattern', () => {
    setDatePatterns({ date: 'JJ.MM', dateTime: '' })
    expect(readDateFnsPattern('date')).toBeNull()
  })

  it('updates every user of the hook at once', () => {
    const first = renderHook(() => useDateFormat())
    const second = renderHook(() => useDateFormat())

    act(() => first.result.current.setDatePatterns({ date: 'YYYY-MM-DD', dateTime: '' }))

    expect(second.result.current.datePattern).toBe('YYYY-MM-DD')
  })
})

describe('applyProfileDatePatterns', () => {
  afterEach(() => {
    localStorage.clear()
    setDatePatterns({ date: '', dateTime: '' })
    vi.restoreAllMocks()
  })

  it('uses the patterns of the profile', async () => {
    const save = vi.fn()
    const { result } = renderHook(() => useDateFormat())

    await act(() => applyProfileDatePatterns({ date_pattern: 'YYYY', date_time_pattern: null }, save))

    expect(result.current.datePattern).toBe('YYYY')
    expect(result.current.dateTimePattern).toBe('')
    expect(save).not.toHaveBeenCalled()
  })

  it('moves locally kept patterns to a profile without patterns', async () => {
    localStorage.setItem(LEGACY.date, 'MM.YY')
    localStorage.setItem(LEGACY.dateTime, 'DD.MM.YYYY HH:mm')
    const save = vi.fn().mockResolvedValue(undefined)

    await applyProfileDatePatterns({}, save)

    expect(save).toHaveBeenCalledWith({ date: 'MM.YY', dateTime: 'DD.MM.YYYY HH:mm' })
    expect(readDateFnsPattern('date')).toBe('MM.yy')
    expect(localStorage.getItem(LEGACY.date)).toBeNull()
    expect(localStorage.getItem(LEGACY.dateTime)).toBeNull()
  })

  it('keeps the local patterns for a retry when saving fails', async () => {
    localStorage.setItem(LEGACY.date, 'MM.YY')
    const save = vi.fn().mockRejectedValue(new Error('offline'))

    await expect(applyProfileDatePatterns({ date_pattern: null }, save)).rejects.toThrow('offline')

    expect(localStorage.getItem(LEGACY.date)).toBe('MM.YY')
    expect(readDateFnsPattern('date')).toBe('MM.yy')
  })

  it('prefers the profile and drops stale local patterns', async () => {
    localStorage.setItem(LEGACY.date, 'MM.YY')
    const save = vi.fn()

    await applyProfileDatePatterns({ date_time_pattern: 'YYYY HH:mm' }, save)

    expect(save).not.toHaveBeenCalled()
    expect(readDateFnsPattern('date')).toBeNull()
    expect(readDateFnsPattern('dateTime')).toBe('yyyy HH:mm')
    expect(localStorage.getItem(LEGACY.date)).toBeNull()
  })

  it('uses the profile when localStorage is unavailable', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new DOMException('blocked', 'SecurityError')
    })
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {
      throw new DOMException('blocked', 'SecurityError')
    })

    await applyProfileDatePatterns({ date_pattern: 'YYYY' }, vi.fn())

    expect(readDateFnsPattern('date')).toBe('yyyy')
  })
})
