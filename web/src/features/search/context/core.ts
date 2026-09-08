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

import { EmailEnvelope } from '@/api'
import { createMailListContext, MailListContextBase } from '@/features/mail-list/context'

export type SearchDialogType = 'mailbox' | 'display' | 'delete' | 'filters' | 'tags' | 'edit-tags' | 'update-tags' | 'restore' | 'delete-mailbox'

export interface SearchContextType extends MailListContextBase<EmailEnvelope, SearchDialogType> {
  editTagsOpen: boolean
  setEditTagsOpen: (open: boolean) => void
}

// A single context instance shared by the provider (context/provider.tsx) and
// the accessor hook (context/index.tsx). Keeping the instantiation here lets
// each of those files export only components / only non-components, which is
// what react-refresh requires.
export const searchContext = createMailListContext<SearchContextType>('useSearchContext')
