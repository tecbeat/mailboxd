import { describe, it, expect } from 'vitest'
import { AxiosError, AxiosHeaders } from 'axios'
import { describeLoginError, describeMfaRestart } from '../login-error'

const t = (key: string) => key

function httpError(status: number): AxiosError {
  return new AxiosError('Request failed', 'ERR_BAD_RESPONSE', undefined, undefined, {
    status,
    statusText: '',
    headers: {},
    config: { headers: new AxiosHeaders() },
    data: {},
  })
}

describe('describeLoginError', () => {
  it('maps 401 to the invalid password message', () => {
    expect(describeLoginError(httpError(401), t)).toEqual({
      title: 'auth.loginFailed',
      description: 'auth.invalidPassword',
    })
  })

  it('maps 429 to the rate limit message', () => {
    expect(describeLoginError(httpError(429), t)).toEqual({
      title: 'auth.loginFailed',
      description: 'auth.tooManyAttempts',
    })
  })

  it('falls back to the raw error message for other failures', () => {
    expect(describeLoginError(new Error('Network Error'), t)).toEqual({
      title: 'auth.somethingWentWrong',
      description: 'Network Error',
    })
  })
})

describe('describeMfaRestart', () => {
  it('maps the too-many-attempts code', () => {
    expect(
      describeMfaRestart(
        { error_code: 'mfa_too_many_attempts', error_message: 'English' },
        t
      )
    ).toBe('auth.mfa.tooManyCodes')
  })

  it('maps the invalid-challenge code', () => {
    expect(
      describeMfaRestart(
        { error_code: 'mfa_challenge_invalid', error_message: 'English' },
        t
      )
    ).toBe('auth.mfa.challengeExpired')
  })

  it('does not match on the English message', () => {
    expect(
      describeMfaRestart(
        { error_message: 'Too many invalid codes. Please sign in again.' },
        t
      )
    ).toBe('Too many invalid codes. Please sign in again.')
  })

  it('passes the message of an unknown code through', () => {
    expect(
      describeMfaRestart(
        { error_code: 'other', error_message: 'Something else' },
        t
      )
    ).toBe('Something else')
  })

  it('falls back to the expired message without a server message', () => {
    expect(
      describeMfaRestart({ error_code: null, error_message: null }, t)
    ).toBe('auth.mfa.challengeExpired')
  })
})
