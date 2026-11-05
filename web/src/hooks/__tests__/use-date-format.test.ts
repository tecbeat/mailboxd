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
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { DATE_PATTERN_STORAGE_KEYS, readDateFnsPattern, useDateFormat } from '../use-date-format'

describe('useDateFormat', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => vi.restoreAllMocks())

  it('defaults to the language format and persists a change', () => {
    const { result } = renderHook(() => useDateFormat())
    expect(result.current.datePattern).toBe('')
    expect(result.current.dateTimePattern).toBe('')
    expect(readDateFnsPattern('date')).toBeNull()

    act(() => result.current.setDatePatterns({ date: ' MM.YY ', dateTime: 'DD.MM.YYYY HH:mm' }))

    expect(result.current.datePattern).toBe('MM.YY')
    expect(result.current.dateTimePattern).toBe('DD.MM.YYYY HH:mm')
    expect(localStorage.getItem(DATE_PATTERN_STORAGE_KEYS.date)).toBe('MM.YY')
    expect(readDateFnsPattern('date')).toBe('MM.yy')
    expect(readDateFnsPattern('dateTime')).toBe('dd.MM.yyyy HH:mm')
  })

  it('removes a pattern that is cleared', () => {
    localStorage.setItem(DATE_PATTERN_STORAGE_KEYS.date, 'MM.YY')
    const { result } = renderHook(() => useDateFormat())

    act(() => result.current.setDatePatterns({ date: '', dateTime: '' }))

    expect(localStorage.getItem(DATE_PATTERN_STORAGE_KEYS.date)).toBeNull()
    expect(result.current.datePattern).toBe('')
  })

  it('falls back to the language format for an invalid stored pattern', () => {
    localStorage.setItem(DATE_PATTERN_STORAGE_KEYS.date, 'JJ.MM')
    expect(readDateFnsPattern('date')).toBeNull()
  })

  it('updates every user of the hook at once', () => {
    const first = renderHook(() => useDateFormat())
    const second = renderHook(() => useDateFormat())

    act(() => first.result.current.setDatePatterns({ date: 'YYYY-MM-DD', dateTime: '' }))

    expect(second.result.current.datePattern).toBe('YYYY-MM-DD')
  })

  it('follows a change made in another tab', () => {
    const { result } = renderHook(() => useDateFormat())

    localStorage.setItem(DATE_PATTERN_STORAGE_KEYS.dateTime, 'YYYY-MM-DD HH:mm')
    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: 'unrelated' }))
    })
    expect(result.current.dateTimePattern).toBe('')

    act(() => {
      window.dispatchEvent(new StorageEvent('storage', { key: DATE_PATTERN_STORAGE_KEYS.dateTime }))
    })
    expect(result.current.dateTimePattern).toBe('YYYY-MM-DD HH:mm')
  })

  it('keeps the choice in memory when localStorage is unavailable', () => {
    const blocked = () => {
      throw new DOMException('blocked', 'SecurityError')
    }
    // Spy on the prototype: jsdom stores properties set on localStorage itself.
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(blocked)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(blocked)
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(blocked)
    const { result } = renderHook(() => useDateFormat())

    act(() => result.current.setDatePatterns({ date: 'MM.YY', dateTime: '' }))
    expect(result.current.datePattern).toBe('MM.YY')

    act(() => result.current.setDatePatterns({ date: '', dateTime: '' }))
    expect(result.current.datePattern).toBe('')

    vi.restoreAllMocks()
    expect(localStorage.length).toBe(0)
  })
})
