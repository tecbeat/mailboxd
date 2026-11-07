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

import { useSyncExternalStore } from 'react'

import { toDateFnsPattern } from '@/lib/date-pattern'

/** `date` is used for plain dates, `dateTime` for dates with a time. */
export type DatePatternKind = 'date' | 'dateTime'

export type DatePatterns = Record<DatePatternKind, string>

/** Where patterns were kept before they moved to the user profile. */
export const LEGACY_DATE_PATTERN_STORAGE_KEYS: Record<DatePatternKind, string> = {
  date: 'mailboxd_date_pattern',
  dateTime: 'mailboxd_date_time_pattern',
}

const listeners = new Set<() => void>()

let patterns: DatePatterns = { date: '', dateTime: '' }

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/**
 * The pattern the user chose for `kind`, in Moment.js tokens, or an empty
 * string for the language default.
 */
export function readDatePattern(kind: DatePatternKind): string {
  return patterns[kind]
}

/** The date-fns pattern for `kind`, or `null` for the language default. */
export function readDateFnsPattern(kind: DatePatternKind): string | null {
  const pattern = readDatePattern(kind)
  return pattern.trim() === '' ? null : toDateFnsPattern(pattern)
}

/** Use `next` for every date in the UI; it is not saved anywhere. */
export function setDatePatterns(next: DatePatterns): void {
  patterns = { date: next.date.trim(), dateTime: next.dateTime.trim() }
  listeners.forEach((listener) => listener())
}

/** The date patterns of the user profile, as sent by the server. */
export interface ProfileDatePatterns {
  date_pattern?: string | null
  date_time_pattern?: string | null
}

function readLegacyPatterns(): DatePatterns | null {
  try {
    const date = localStorage.getItem(LEGACY_DATE_PATTERN_STORAGE_KEYS.date)?.trim() ?? ''
    const dateTime = localStorage.getItem(LEGACY_DATE_PATTERN_STORAGE_KEYS.dateTime)?.trim() ?? ''
    return date || dateTime ? { date, dateTime } : null
  } catch {
    return null
  }
}

function removeLegacyPatterns(): void {
  try {
    Object.values(LEGACY_DATE_PATTERN_STORAGE_KEYS).forEach((key) => localStorage.removeItem(key))
  } catch {
    // localStorage unavailable; there is nothing to remove.
  }
}

/**
 * Use the patterns of the user profile. Patterns this browser kept locally
 * before they moved to the profile are taken over once, when the profile
 * has none, and saved with `save`. They stay in localStorage until `save`
 * succeeds, so a failed upload is retried on the next load.
 */
export async function applyProfileDatePatterns(
  profile: ProfileDatePatterns,
  save: (legacy: DatePatterns) => Promise<unknown>,
): Promise<void> {
  const legacy = readLegacyPatterns()
  if (legacy && !profile.date_pattern && !profile.date_time_pattern) {
    setDatePatterns(legacy)
    await save(legacy)
  } else {
    setDatePatterns({ date: profile.date_pattern ?? '', dateTime: profile.date_time_pattern ?? '' })
  }
  removeLegacyPatterns()
}

/**
 * The date patterns of every absolute date in the UI. Every component using
 * this hook re-renders when a pattern changes.
 */
export function useDateFormat() {
  const datePattern = useSyncExternalStore(subscribe, () => readDatePattern('date'), () => '')
  const dateTimePattern = useSyncExternalStore(subscribe, () => readDatePattern('dateTime'), () => '')
  return { datePattern, dateTimePattern, setDatePatterns }
}
