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


import { AttachmentModel } from '@/api/attachment/api'
import { createMailListContext, MailListContextBase } from '@/features/mail-list/context'

export type AttachmentDialogType = 'mailbox' | 'display' | 'delete' | 'filters' | 'tags' | 'edit-tags' | 'update-tags' | 'restore' | 'delete-mailbox' | 'nested-eml'

export type AttachmentContextType = MailListContextBase<AttachmentModel, AttachmentDialogType>

const { Provider, useMailListContext } = createMailListContext<AttachmentContextType>('useAttachmentContext')

export default Provider
export const useAttachmentContext = useMailListContext
