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
import { createMailListContext } from '../context'

interface FooValue {
  greeting: string
}

describe('createMailListContext', () => {
  it('provides the value to consumers inside the provider', () => {
    const { Provider, useMailListContext } = createMailListContext<FooValue>('useFoo')

    function Consumer() {
      const { greeting } = useMailListContext()
      return <span>{greeting}</span>
    }

    render(
      <Provider value={{ greeting: 'hello world' }}>
        <Consumer />
      </Provider>,
    )

    expect(screen.getByText('hello world')).toBeInTheDocument()
  })

  it('throws a named error when used outside its provider', () => {
    const { useMailListContext } = createMailListContext<FooValue>('useFoo')
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    expect(() => renderHook(() => useMailListContext())).toThrow(
      'useFoo has to be used within its provider',
    )

    spy.mockRestore()
  })

  it('isolates separate context instances', () => {
    const first = createMailListContext<FooValue>('useFirst')
    const second = createMailListContext<FooValue>('useSecond')
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    function Consumer() {
      const { greeting } = second.useMailListContext()
      return <span>{greeting}</span>
    }

    // Providing the first context must not satisfy the second hook.
    expect(() =>
      render(
        <first.Provider value={{ greeting: 'from first' }}>
          <Consumer />
        </first.Provider>,
      ),
    ).toThrow('useSecond has to be used within its provider')

    spy.mockRestore()
  })
})
