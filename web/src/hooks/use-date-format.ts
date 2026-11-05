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

import { useCallback, useSyncExternalStore } from 'react'

import { toDateFnsPattern } from '@/lib/date-pattern'

/** `date` is used for plain dates, `dateTime` for dates with a time. */
export type DatePatternKind = 'date' | 'dateTime'

export const DATE_PATTERN_STORAGE_KEYS: Record<DatePatternKind, string> = {
  date: 'mailboxd_date_pattern',
  dateTime: 'mailboxd_date_time_pattern',
}

const listeners = new Set<() => void>()

const memoryPatterns: Record<DatePatternKind, string> = { date: '', dateTime: '' }

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  const keys = Object.values(DATE_PATTERN_STORAGE_KEYS)
  const onStorage = (event: StorageEvent) => {
    if (event.key !== null && keys.includes(event.key)) listener()
  }
  window.addEventListener('storage', onStorage)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('storage', onStorage)
  }
}

/**
 * The pattern the user entered for `kind`, in Moment.js tokens, or an empty
 * string for the language default.
 */
export function readDatePattern(kind: DatePatternKind): string {
  try {
    return localStorage.getItem(DATE_PATTERN_STORAGE_KEYS[kind]) ?? ''
  } catch {
    return memoryPatterns[kind]
  }
}

/** The date-fns pattern for `kind`, or `null` for the language default. */
export function readDateFnsPattern(kind: DatePatternKind): string | null {
  const pattern = readDatePattern(kind)
  return pattern.trim() === '' ? null : toDateFnsPattern(pattern)
}

function store(kind: DatePatternKind, pattern: string): void {
  const value = pattern.trim()
  memoryPatterns[kind] = value
  try {
    if (value === '') {
      localStorage.removeItem(DATE_PATTERN_STORAGE_KEYS[kind])
    } else {
      localStorage.setItem(DATE_PATTERN_STORAGE_KEYS[kind], value)
    }
  } catch {
    // localStorage unavailable; the choice lasts until the page is reloaded.
  }
}

/**
 * The date patterns of every absolute date in the UI. Every component using
 * this hook re-renders when a pattern changes.
 */
export function useDateFormat() {
  const datePattern = useSyncExternalStore(subscribe, () => readDatePattern('date'), () => '')
  const dateTimePattern = useSyncExternalStore(subscribe, () => readDatePattern('dateTime'), () => '')

  const setDatePatterns = useCallback((patterns: Record<DatePatternKind, string>) => {
    store('date', patterns.date)
    store('dateTime', patterns.dateTime)
    listeners.forEach((listener) => listener())
  }, [])

  return { datePattern, dateTimePattern, setDatePatterns }
}
