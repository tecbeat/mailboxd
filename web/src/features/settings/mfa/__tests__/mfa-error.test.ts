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
import { AxiosError, AxiosHeaders } from 'axios'
import { describe, expect, it } from 'vitest'
import { describeMfaError } from '../mfa-error'

const t = (key: string) => key

function httpError(status: number, data: unknown = {}): AxiosError {
  return new AxiosError(
    'Request failed',
    'ERR_BAD_RESPONSE',
    undefined,
    undefined,
    {
      status,
      statusText: '',
      headers: {},
      config: { headers: new AxiosHeaders() },
      data,
    }
  )
}

describe('describeMfaError', () => {
  it('maps 400 to the invalid code message', () => {
    expect(describeMfaError(httpError(400), t)).toBe(
      'settings.mfa.errors.invalidCode'
    )
  })

  it('maps 429 to the rate limit message', () => {
    expect(describeMfaError(httpError(429), t)).toBe(
      'settings.mfa.errors.tooManyAttempts'
    )
  })

  it('uses the server message for other HTTP errors', () => {
    expect(
      describeMfaError(
        httpError(409, { code: 1, message: 'Already enabled' }),
        t
      )
    ).toBe('Already enabled')
  })

  it('falls back to the error message', () => {
    expect(describeMfaError(httpError(500), t)).toBe('Request failed')
    expect(describeMfaError(new Error('Network Error'), t)).toBe(
      'Network Error'
    )
  })
})
