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

import { describe, it, expect, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { render, screen } from '@/test/test-utils'
import { MailListConfigProvider, useMailListConfig, type ListContextHook } from '../config'

describe('useMailListConfig', () => {
  it('exposes the feature list-context hook supplied by the provider', () => {
    const useListContext = vi.fn() as unknown as ListContextHook
    let seen: ListContextHook | undefined

    function Consumer() {
      seen = useMailListConfig().useListContext
      return <span>ok</span>
    }

    render(
      <MailListConfigProvider config={{ useListContext }}>
        <Consumer />
      </MailListConfigProvider>,
    )

    expect(screen.getByText('ok')).toBeInTheDocument()
    expect(seen).toBe(useListContext)
  })

  it('throws when used outside a provider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    expect(() => renderHook(() => useMailListConfig())).toThrow(
      'useMailListConfig has to be used within a MailListConfigProvider',
    )

    spy.mockRestore()
  })
})
