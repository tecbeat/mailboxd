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
import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { MfaStatus } from '@/api/users/api'
import { SettingsMfa } from '../index'

type BlockerOptions = {
  shouldBlockFn: () => boolean
  enableBeforeUnload: boolean
  disabled: boolean
}

const { toast, saveAs, useBlocker } = vi.hoisted(() => ({
  toast: vi.fn(),
  saveAs: vi.fn(),
  useBlocker: vi.fn<(options: BlockerOptions) => void>(),
}))

vi.mock('@tanstack/react-router', () => ({ useBlocker }))
vi.mock('@/hooks/use-toast', () => ({ toast }))
vi.mock('file-saver', () => ({ saveAs }))

const BASE = 'http://localhost:15630/api/v1/mfa'
const CODES = Array.from(
  { length: 10 },
  (_, i) => `ABCD23${String(i).padStart(2, '0')}`
)

function withStatus(status: Partial<MfaStatus>) {
  const full: MfaStatus = {
    enabled: false,
    available: true,
    recovery_codes_left: 0,
    ...status,
  }
  server.use(http.get(`${BASE}/status`, () => HttpResponse.json(full)))
}

function lastBlocker(): BlockerOptions {
  const calls = useBlocker.mock.calls
  return calls[calls.length - 1][0]
}

async function startEnrolment(user: ReturnType<typeof userEvent.setup>) {
  withStatus({})
  server.use(
    http.post(`${BASE}/enroll`, () =>
      HttpResponse.json({
        secret: 'JBSWY3DPEHPK3PXP',
        otpauth_uri:
          'otpauth://totp/mailboxd:admin?secret=JBSWY3DPEHPK3PXP&issuer=mailboxd',
      })
    )
  )
  renderWithProviders(<SettingsMfa />)
  await user.click(
    await screen.findByRole('button', {
      name: 'Enable two-factor authentication',
    })
  )
  await screen.findByText('JBSWY3DPEHPK3PXP')
}

describe('SettingsMfa', () => {
  beforeEach(() => {
    toast.mockReset()
    saveAs.mockReset()
    useBlocker.mockReset()
  })

  it('explains that SSO users cannot enrol here', async () => {
    withStatus({ available: false })
    renderWithProviders(<SettingsMfa />)

    expect(
      await screen.findByText(/handled by your identity provider/)
    ).toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: 'Enable two-factor authentication' })
    ).not.toBeInTheDocument()
  })

  it('shows an error when the status cannot be loaded', async () => {
    server.use(
      http.get(`${BASE}/status`, () => HttpResponse.json({}, { status: 500 }))
    )
    renderWithProviders(<SettingsMfa />)

    expect(
      await screen.findByText(
        'Failed to load the two-factor authentication status.'
      )
    ).toBeInTheDocument()
  })

  it('offers to enable two-factor authentication when it is off', async () => {
    withStatus({})
    renderWithProviders(<SettingsMfa />)

    expect(
      await screen.findByText('Two-factor authentication is off')
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Enable two-factor authentication' })
    ).toBeEnabled()
  })

  it('shows the remaining recovery codes when it is on', async () => {
    withStatus({ enabled: true, recovery_codes_left: 7 })
    renderWithProviders(<SettingsMfa />)

    expect(
      await screen.findByText('Two-factor authentication is on')
    ).toBeInTheDocument()
    expect(screen.getByText('Unused recovery codes: 7')).toBeInTheDocument()
    expect(
      screen.queryByText(/running out of recovery codes/)
    ).not.toBeInTheDocument()
  })

  it('warns when recovery codes run low', async () => {
    withStatus({ enabled: true, recovery_codes_left: 2 })
    renderWithProviders(<SettingsMfa />)

    expect(
      await screen.findByText(/running out of recovery codes/)
    ).toBeInTheDocument()
  })

  it('shows a toast when the enrolment cannot be started', async () => {
    withStatus({})
    server.use(
      http.post(`${BASE}/enroll`, () =>
        HttpResponse.json(
          { code: 40000, message: 'Already enabled' },
          { status: 409 }
        )
      )
    )
    const user = userEvent.setup()
    renderWithProviders(<SettingsMfa />)
    await user.click(
      await screen.findByRole('button', {
        name: 'Enable two-factor authentication',
      })
    )

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith({
        variant: 'destructive',
        title: 'Could not start the setup',
        description: 'Already enabled',
      })
    )
  })

  it('shows the QR code and secret and copies the secret', async () => {
    const user = userEvent.setup()
    await startEnrolment(user)

    const qr = await screen.findByAltText('QR code for your authenticator app')
    expect(qr.getAttribute('src')).toMatch(/^data:image\/svg\+xml/)
    await user.click(screen.getByRole('button', { name: 'Copy secret' }))

    expect(await navigator.clipboard.readText()).toBe('JBSWY3DPEHPK3PXP')
    expect(toast).toHaveBeenCalledWith({ title: 'Copied to clipboard' })
  })

  it('reports when copying fails', async () => {
    const user = userEvent.setup()
    await startEnrolment(user)
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValueOnce(
      new Error('denied')
    )
    await user.click(screen.getByRole('button', { name: 'Copy secret' }))

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(
        expect.objectContaining({ variant: 'destructive' })
      )
    )
  })

  it('keeps the enrolment open after a wrong code', async () => {
    const user = userEvent.setup()
    await startEnrolment(user)
    server.use(
      http.post(`${BASE}/confirm`, () =>
        HttpResponse.json(
          { code: 40000, message: 'Invalid code' },
          { status: 400 }
        )
      )
    )
    await user.type(screen.getByLabelText('Code from the app'), '000000')
    await user.click(screen.getByRole('button', { name: 'Confirm and enable' }))

    expect(
      await screen.findByText('Invalid verification code. Please try again.')
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Code from the app')).toHaveAttribute(
      'aria-invalid',
      'true'
    )
    expect(screen.getByText('JBSWY3DPEHPK3PXP')).toBeInTheDocument()
  })

  it('cancels the enrolment', async () => {
    const user = userEvent.setup()
    await startEnrolment(user)
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(
      await screen.findByText('Two-factor authentication is off')
    ).toBeInTheDocument()
  })

  it('shows ten recovery codes once enabled and lets the user save them', async () => {
    const user = userEvent.setup()
    await startEnrolment(user)
    let body: unknown
    server.use(
      http.post(`${BASE}/confirm`, async ({ request }) => {
        body = await request.json()
        withStatus({ enabled: true, recovery_codes_left: 10 })
        return HttpResponse.json({ recovery_codes: CODES })
      })
    )
    await user.type(screen.getByLabelText('Code from the app'), ' 123456 ')
    await user.click(screen.getByRole('button', { name: 'Confirm and enable' }))

    expect(
      await screen.findByText('Save your recovery codes')
    ).toBeInTheDocument()
    expect(body).toEqual({ code: '123456' })
    expect(screen.getAllByRole('listitem')).toHaveLength(10)

    // Leaving is guarded until the user confirms the codes are saved.
    expect(lastBlocker()).toMatchObject({
      disabled: false,
      enableBeforeUnload: true,
    })
    const confirm = vi.spyOn(window, 'confirm').mockReturnValueOnce(false)
    expect(lastBlocker().shouldBlockFn()).toBe(true)
    expect(confirm).toHaveBeenCalledWith(
      'You have not confirmed that you saved your recovery codes. They will not be shown again. Leave anyway?'
    )

    await user.click(screen.getByRole('button', { name: 'Copy codes' }))
    expect(await navigator.clipboard.readText()).toBe(CODES.join('\n'))

    await user.click(screen.getByRole('button', { name: 'Download as .txt' }))
    expect(saveAs).toHaveBeenCalledWith(
      expect.any(Blob),
      'mailboxd-recovery-codes.txt'
    )
    const blob = saveAs.mock.calls[0][0] as Blob
    expect(await blob.text()).toBe(`${CODES.join('\n')}\n`)

    const done = screen.getByRole('button', { name: 'Done' })
    expect(done).toBeDisabled()
    await user.click(
      screen.getByLabelText('I have saved my recovery codes in a safe place')
    )
    expect(lastBlocker()).toMatchObject({
      disabled: true,
      enableBeforeUnload: false,
    })
    await user.click(done)

    expect(
      await screen.findByText('Two-factor authentication is on')
    ).toBeInTheDocument()
    expect(screen.getByText('Unused recovery codes: 10')).toBeInTheDocument()
  })

  it('keeps two-factor authentication on after a wrong disable code', async () => {
    withStatus({ enabled: true, recovery_codes_left: 5 })
    server.use(
      http.post(`${BASE}/disable`, () =>
        HttpResponse.json(
          { code: 40000, message: 'Invalid code' },
          { status: 400 }
        )
      )
    )
    const user = userEvent.setup()
    renderWithProviders(<SettingsMfa />)
    await user.click(
      await screen.findByRole('button', {
        name: 'Disable two-factor authentication',
      })
    )
    await user.type(
      screen.getByLabelText(
        'Code from your authenticator app or a recovery code'
      ),
      '000000'
    )
    await user.click(screen.getByRole('button', { name: 'Disable' }))

    expect(
      await screen.findByText('Invalid verification code. Please try again.')
    ).toBeInTheDocument()
    expect(toast).not.toHaveBeenCalled()

    // Cancelling clears the error for the next attempt.
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    await user.click(
      screen.getByRole('button', { name: 'Disable two-factor authentication' })
    )
    expect(
      screen.queryByText('Invalid verification code. Please try again.')
    ).not.toBeInTheDocument()
  })

  it('disables two-factor authentication with the right code', async () => {
    withStatus({ enabled: true, recovery_codes_left: 5 })
    let body: unknown
    server.use(
      http.post(`${BASE}/disable`, async ({ request }) => {
        body = await request.json()
        withStatus({ enabled: false })
        return new HttpResponse(null, { status: 200 })
      })
    )
    const user = userEvent.setup()
    renderWithProviders(<SettingsMfa />)
    await user.click(
      await screen.findByRole('button', {
        name: 'Disable two-factor authentication',
      })
    )
    await user.type(
      screen.getByLabelText(
        'Code from your authenticator app or a recovery code'
      ),
      '123456'
    )
    await user.click(screen.getByRole('button', { name: 'Disable' }))

    expect(
      await screen.findByText('Two-factor authentication is off')
    ).toBeInTheDocument()
    expect(body).toEqual({ code: '123456' })
    expect(toast).toHaveBeenCalledWith({
      title: 'Two-factor authentication disabled',
    })
  })
})
