//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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

import { useCallback, useSyncExternalStore } from 'react'

/** `locale` follows the UI language, `iso` is ISO 8601 (YYYY-MM-DD). */
export type DateFormat = 'locale' | 'iso'

export const DATE_FORMAT_STORAGE_KEY = 'mailboxd_date_format'

const DEFAULT_FORMAT: DateFormat = 'locale'

const listeners = new Set<() => void>()

let memoryFormat: DateFormat = DEFAULT_FORMAT

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  const onStorage = (event: StorageEvent) => {
    if (event.key === DATE_FORMAT_STORAGE_KEY) listener()
  }
  window.addEventListener('storage', onStorage)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('storage', onStorage)
  }
}

/** The chosen date format, persisted in localStorage when available. */
export function readDateFormat(): DateFormat {
  try {
    const saved = localStorage.getItem(DATE_FORMAT_STORAGE_KEY)
    return saved === 'iso' || saved === 'locale' ? saved : DEFAULT_FORMAT
  } catch {
    return memoryFormat
  }
}

/**
 * The date format of every absolute date in the UI. Every component using
 * this hook re-renders when the format changes.
 */
export function useDateFormat() {
  const dateFormat = useSyncExternalStore(subscribe, readDateFormat, () => DEFAULT_FORMAT)

  const setDateFormat = useCallback((format: DateFormat) => {
    memoryFormat = format
    try {
      localStorage.setItem(DATE_FORMAT_STORAGE_KEY, format)
    } catch {
      // localStorage unavailable; the choice lasts until the page is reloaded.
    }
    listeners.forEach((listener) => listener())
  }, [])

  return { dateFormat, setDateFormat }
}
