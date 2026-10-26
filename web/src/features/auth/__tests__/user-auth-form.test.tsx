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

import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

const { navigate, toast } = vi.hoisted(() => ({ navigate: vi.fn(), toast: vi.fn() }))

vi.mock('@tanstack/react-router', () => ({
  useLocation: () => ({ search: {} }),
  useNavigate: () => navigate,
}))
vi.mock('@/hooks/use-toast', () => ({ toast }))
vi.mock('@/context/theme-context', () => ({ useTheme: () => ({ setTheme: vi.fn() }) }))
vi.mock('@/hooks/use-oidc-config', () => ({
  useOidcConfig: () => ({ oidcEnabled: false, oidcAutoRedirect: false }),
}))

import { getToken } from '@/stores/authStore'
import { UserAuthForm } from '../user-auth-form'

const LOGIN_URL = 'http://localhost:15630/api/login'
const VERIFY_URL = 'http://localhost:15630/api/auth/mfa/verify'

function requireMfa() {
  server.use(
    http.post(LOGIN_URL, () =>
      HttpResponse.json({
        success: true,
        mfa_required: true,
        mfa_challenge: 'challenge-1',
      })
    )
  )
}

async function enterCode(code: string) {
  const input = await screen.findByLabelText('Verification code')
  await userEvent.clear(input)
  await userEvent.type(input, code)
  await userEvent.click(screen.getByRole('button', { name: 'Verify' }))
}

async function logIn() {
  renderWithProviders(<UserAuthForm />)
  await userEvent.type(screen.getByLabelText('Username'), 'admin')
  await userEvent.type(screen.getByPlaceholderText('********'), 'wrong-password')
  await userEvent.click(screen.getByRole('button', { name: 'Login' }))
}

describe('UserAuthForm', () => {
  beforeEach(() => {
    toast.mockReset()
    navigate.mockReset()
    localStorage.clear()
  })

  it('signs in directly when no second factor is required', async () => {
    server.use(
      http.post(LOGIN_URL, () =>
        HttpResponse.json({
          success: true,
          access_token: 'token-plain',
          language: 'en',
          theme: 'dark',
        })
      )
    )
    await logIn()

    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ href: '/' }))
    expect(getToken()?.accessToken).toBe('token-plain')
  })

  it('asks for a code after the password and signs in with it', async () => {
    requireMfa()
    let body: unknown
    server.use(
      http.post(VERIFY_URL, async ({ request }) => {
        body = await request.json()
        return HttpResponse.json({
          success: true,
          access_token: 'token-mfa',
          language: 'en',
        })
      })
    )
    await logIn()
    expect(screen.queryByLabelText('Username')).not.toBeInTheDocument()
    await enterCode('123 456')

    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ href: '/' }))
    expect(body).toEqual({ challenge: 'challenge-1', code: '123 456' })
    expect(getToken()?.accessToken).toBe('token-mfa')
  })

  it('accepts a recovery code in the second step', async () => {
    requireMfa()
    let body: { code?: string } = {}
    server.use(
      http.post(VERIFY_URL, async ({ request }) => {
        body = (await request.json()) as { code?: string }
        return HttpResponse.json({
          success: true,
          access_token: 'token-rc',
          language: 'en',
        })
      })
    )
    await logIn()
    await enterCode('ABCD-EFGH')

    await waitFor(() => expect(navigate).toHaveBeenCalled())
    expect(body.code).toBe('ABCD-EFGH')
  })

  it('rejects malformed codes before sending them', async () => {
    requireMfa()
    const verify = vi.fn()
    server.use(http.post(VERIFY_URL, verify))
    await logIn()
    await enterCode('12')

    expect(
      await screen.findByText(
        'Enter the 6-digit code from your authenticator app or a recovery code.'
      )
    ).toBeInTheDocument()
    expect(verify).not.toHaveBeenCalled()
  })

  it('stays on the code step after a wrong code', async () => {
    requireMfa()
    server.use(
      http.post(VERIFY_URL, () =>
        HttpResponse.json({
          success: false,
          mfa_required: true,
          error_code: 'mfa_invalid_code',
          error_message: 'Invalid verification code.',
        })
      )
    )
    await logIn()
    await enterCode('000000')

    expect(
      await screen.findByText('Invalid verification code. Please try again.')
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Verification code')).toHaveValue('')
    expect(navigate).not.toHaveBeenCalled()
    expect(toast).not.toHaveBeenCalled()
  })

  it('goes back to the password step after too many wrong codes', async () => {
    requireMfa()
    server.use(
      http.post(VERIFY_URL, () =>
        HttpResponse.json({
          success: false,
          error_code: 'mfa_too_many_attempts',
          error_message: 'Server text',
        })
      )
    )
    await logIn()
    await enterCode('000000')

    expect(await screen.findByLabelText('Username')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('********')).toHaveValue('')
    expect(toast).toHaveBeenCalledWith(
      expect.objectContaining({
        variant: 'destructive',
        title: 'Login Failed',
        description: 'Too many invalid codes. Please sign in again.',
      })
    )
  })

  it('goes back to the password step when the sign-in expired', async () => {
    requireMfa()
    server.use(
      http.post(VERIFY_URL, () =>
        HttpResponse.json({
          success: false,
          error_code: 'mfa_challenge_invalid',
          error_message:
            'The sign-in has expired or is no longer valid. Please sign in again.',
        })
      )
    )
    await logIn()
    await enterCode('123456')

    expect(await screen.findByLabelText('Username')).toBeInTheDocument()
    expect(toast).toHaveBeenCalledWith(
      expect.objectContaining({
        description: 'The sign-in has expired. Please sign in again.',
      })
    )
  })

  it('keeps the code step when verification is rate-limited', async () => {
    requireMfa()
    server.use(
      http.post(VERIFY_URL, () =>
        HttpResponse.json({ message: 'slow down' }, { status: 429 })
      )
    )
    await logIn()
    await enterCode('123456')

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: 'destructive',
          title: 'Login Failed',
          description:
            'Too many login attempts. Please wait a few minutes and try again.',
        })
      )
    )
    expect(screen.getByLabelText('Verification code')).toBeInTheDocument()
  })

  it('returns to the password step with the back button', async () => {
    requireMfa()
    await logIn()
    await userEvent.click(
      await screen.findByRole('button', { name: 'Back to sign-in' })
    )

    expect(await screen.findByLabelText('Username')).toHaveValue('admin')
    expect(screen.getByPlaceholderText('********')).toHaveValue('')
  })

  it('tells the user to wait when login is rate-limited', async () => {
    server.use(
      http.post('http://localhost:15630/api/login', () =>
        HttpResponse.json({ message: 'rate limited' }, { status: 429 }),
      ),
    )
    await logIn()

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({
          variant: 'destructive',
          title: 'Login Failed',
          description: 'Too many login attempts. Please wait a few minutes and try again.',
        }),
      ),
    )
    expect(navigate).not.toHaveBeenCalled()
    // The form is usable again for the next attempt.
    expect(screen.getByRole('button', { name: 'Login' })).toBeEnabled()
  })
})
