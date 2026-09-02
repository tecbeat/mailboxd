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

import type { TFunction } from 'i18next'
import { z } from 'zod'

/**
 * Build the access-token form schema with translated validation messages.
 * Shared by the settings and user-management token dialogs so their validation
 * rules cannot drift apart again.
 */
export const getAccessTokenSchema = (t: TFunction) =>
  z.object({
    name: z
      .string()
      .max(32, t('apiTokens.form.errorMax'))
      .optional()
      .or(z.literal('')),
    expire_in: z
      .number({
        error: t('apiTokens.form.errorNumber'),
      })
      .int(t('apiTokens.form.errorInt'))
      .positive(t('apiTokens.form.errorPositive'))
      .optional(),
  })

export type AccessTokenForm = z.infer<ReturnType<typeof getAccessTokenSchema>>
