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
import { describe, expect, it, vi } from 'vitest'

import { server } from '@/test/server'
import { renderWithProviders } from '@/test/utils'

import type { OAuth2Entity } from '../../data/schema'
import { ActionDialog } from '../action-dialog'

const BASE = 'http://localhost:15630'

// jsdom lacks the layout and pointer APIs the Radix scroll area and select use.
Element.prototype.hasPointerCapture ??= () => false
Element.prototype.releasePointerCapture ??= () => {}
Element.prototype.scrollIntoView ??= () => {}
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
)

const entity: OAuth2Entity = {
  id: 9,
  description: 'Microsoft',
  client_id: 'client-9',
  client_secret: 'secret',
  auth_url: 'https://login.example.com/authorize',
  token_url: 'https://login.example.com/token',
  redirect_uri: 'https://mailboxd.example.com/oauth2/callback',
  scopes: ['offline_access'],
  enabled: true,
  use_proxy: 3,
  created_at: 0,
  updated_at: 0,
}

function proxies(...items: { id: number; url: string }[]) {
  server.use(
    http.get(`${BASE}/api/v1/list-proxy`, () =>
      HttpResponse.json(items.map((p) => ({ ...p, created_at: 0, updated_at: 0 }))),
    ),
  )
}

/** Picks a proxy option and saves; returns the body sent to the server. */
async function saveWithProxy(option: string): Promise<Record<string, unknown>> {
  let body: Record<string, unknown> | undefined
  server.use(
    http.post(`${BASE}/api/v1/oauth2/9`, async ({ request }) => {
      body = (await request.json()) as Record<string, unknown>
      return HttpResponse.json(entity)
    }),
  )
  const user = userEvent.setup()
  renderWithProviders(<ActionDialog open onOpenChange={vi.fn()} currentRow={entity} />)

  await user.click(await screen.findByRole('combobox'))
  await user.click(await screen.findByRole('option', { name: option }))
  await user.click(screen.getByRole('button', { name: 'Save changes' }))

  await waitFor(() => expect(body).toBeDefined())
  return body!
}

describe('OAuth2 ActionDialog proxy select', () => {
  it('clears the stored proxy when No proxy is chosen', async () => {
    proxies({ id: 3, url: 'socks5://proxy-a:1080' })
    const body = await saveWithProxy('No proxy')

    expect(body.clear_proxy).toBe(true)
    expect(body).not.toHaveProperty('use_proxy')
  })

  it('sends the id of a chosen proxy', async () => {
    proxies({ id: 3, url: 'socks5://proxy-a:1080' }, { id: 4, url: 'socks5://proxy-b:1080' })
    const body = await saveWithProxy('socks5://proxy-b:1080')

    expect(body.use_proxy).toBe(4)
    expect(body).not.toHaveProperty('clear_proxy')
  })
})
