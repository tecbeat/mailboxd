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

import React from 'react'
import { SortingState } from '@tanstack/react-table'

// Fields shared by every mail-list-style feature (search, attachment, ...).
// TEntity is the row entity (e.g. EmailEnvelope, AttachmentModel) and TDialog
// is the union of dialog identifiers the feature can open.
export interface MailListContextBase<TEntity, TDialog extends string> {
  open: TDialog | null
  setOpen: (str: TDialog | null) => void
  currentItem: TEntity | undefined
  setCurrentItem: React.Dispatch<React.SetStateAction<TEntity | undefined>>
  toDelete: Map<number, Set<string>>
  setToDelete: React.Dispatch<React.SetStateAction<Map<number, Set<string>>>>
  selected: Map<number, Set<string>>
  setSelected: React.Dispatch<React.SetStateAction<Map<number, Set<string>>>>
  deleteMailboxId: string | undefined
  setDeleteMailboxId: React.Dispatch<React.SetStateAction<string | undefined>>
  selectedAccountId: number | undefined
  setSelectedAccountId: React.Dispatch<React.SetStateAction<number | undefined>>
  selectedTags: string[]
  sorting: SortingState
  setSorting: React.Dispatch<React.SetStateAction<SortingState>>
  // The filter shape is feature-specific and accessed with dynamic keys; keep
  // it loose to match the existing behaviour of both features.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  filter: Record<string, any>
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  setFilter: React.Dispatch<React.SetStateAction<Record<string, any>>>
  handleTagToggle: (tag: string) => void
}

interface ProviderProps<TValue> {
  children: React.ReactNode
  value: TValue
}

// Builds an isolated React context plus its provider and typed accessor hook.
// Each call returns a fresh context object, so distinct features never share
// state even though they share this factory.
export function createMailListContext<TValue>(hookName: string) {
  const Context = React.createContext<TValue | null>(null)

  function Provider({ children, value }: ProviderProps<TValue>) {
    return <Context.Provider value={value}>{children}</Context.Provider>
  }

  function useMailListContext(): TValue {
    const context = React.useContext(Context)

    if (!context) {
      throw new Error(`${hookName} has to be used within its provider`)
    }

    return context
  }

  return { Provider, useMailListContext }
}
