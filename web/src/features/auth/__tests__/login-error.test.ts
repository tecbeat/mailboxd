import { describe, it, expect } from 'vitest'
import { AxiosError, AxiosHeaders } from 'axios'
import { describeLoginError } from '../login-error'

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
