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
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import { useCallback, useSyncExternalStore } from 'react'

export type DateDisplayMode = 'relative' | 'absolute'

export const DATE_DISPLAY_STORAGE_KEY = 'mailboxd_date_display'

const DEFAULT_MODE: DateDisplayMode = 'relative'

const listeners = new Set<() => void>()

// Used when localStorage is unavailable (e.g. blocked by the browser).
let memoryMode: DateDisplayMode = DEFAULT_MODE

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  // Keep other tabs in sync.
  const onStorage = (event: StorageEvent) => {
    if (event.key === DATE_DISPLAY_STORAGE_KEY) listener()
  }
  window.addEventListener('storage', onStorage)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('storage', onStorage)
  }
}

function readMode(): DateDisplayMode {
  try {
    const saved = localStorage.getItem(DATE_DISPLAY_STORAGE_KEY)
    return saved === 'absolute' || saved === 'relative' ? saved : DEFAULT_MODE
  } catch {
    return memoryMode
  }
}

/**
 * How the date column of the search and attachment lists is shown. Every
 * component using this hook re-renders when the mode changes; the choice is
 * persisted in localStorage when available.
 */
export function useDateDisplay() {
  const dateDisplay = useSyncExternalStore(subscribe, readMode, () => DEFAULT_MODE)

  const setDateDisplay = useCallback((mode: DateDisplayMode) => {
    memoryMode = mode
    try {
      localStorage.setItem(DATE_DISPLAY_STORAGE_KEY, mode)
    } catch {
      // localStorage unavailable; the choice lasts until the page is reloaded.
    }
    listeners.forEach((listener) => listener())
  }, [])

  return { dateDisplay, setDateDisplay }
}
