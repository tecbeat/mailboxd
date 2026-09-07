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

import React from 'react'
import { EmailEnvelope } from '@/api'
import { MailListContextBase } from './context'

// A feature's list context accessor, entity- and dialog-agnostic. Concrete
// features (search, attachment) provide their own useSearchContext /
// useAttachmentContext here; shared components read it through the config so a
// single component can serve every feature.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ListContextHook = () => MailListContextBase<any, any>

// Resolves the feature's current list item to a full email envelope. Search's
// item already is an envelope, so it resolves synchronously; attachment's item
// only references one, so it fetches. Shared message/dialog components read the
// envelope through this hook and stay agnostic of the underlying entity.
export type CurrentEnvelopeHook = () => {
  data: EmailEnvelope | undefined
  isLoading: boolean
  error: unknown
}

export interface MailListConfig {
  useListContext: ListContextHook
  useCurrentEnvelope: CurrentEnvelopeHook
}

export const MailListConfigContext = React.createContext<MailListConfig | null>(null)

export function useMailListConfig(): MailListConfig {
  const config = React.useContext(MailListConfigContext)

  if (!config) {
    throw new Error('useMailListConfig has to be used within a MailListConfigProvider')
  }

  return config
}
