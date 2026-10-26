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
import { AxiosError } from 'axios'

/** Map a failed two-factor settings request to the message to show. */
export function describeMfaError(
  error: unknown,
  t: (key: string) => string
): string {
  if (error instanceof AxiosError) {
    const status = error.response?.status
    if (status === 400) {
      return t('settings.mfa.errors.invalidCode')
    }
    if (status === 429) {
      return t('settings.mfa.errors.tooManyAttempts')
    }
    const data = error.response?.data as { message?: string } | undefined
    if (data?.message) {
      return data.message
    }
  }
  return (error as Error).message
}
