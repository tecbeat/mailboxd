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

import { useState } from 'react'
import { type SortingState } from '@tanstack/react-table'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { render, screen } from '@/test/test-utils'
import { MailListConfigProvider } from '../config-provider'
import { type ListContextHook } from '../config'
import { RelevanceSortButton } from '../relevance-sort'
import { DEFAULT_SORTING, initialSorting, sortParams } from '../sorting'

function Harness({ filter, initial }: { filter: Record<string, unknown>; initial: SortingState }) {
  const [sorting, setSorting] = useState(initial)
  const useListContext = (() => ({ filter, sorting, setSorting })) as unknown as ListContextHook
  return (
    <MailListConfigProvider config={{ useListContext, useCurrentEnvelope: vi.fn() }}>
      <RelevanceSortButton textFields={['text', 'subject']} />
      <output data-testid='sorting'>{JSON.stringify(sorting)}</output>
    </MailListConfigProvider>
  )
}

const sorting = () => JSON.parse(screen.getByTestId('sorting').textContent ?? 'null')
const button = () => screen.getByRole('button', { name: 'Relevance' })

describe('RelevanceSortButton', () => {
  it('is unavailable without a text query', () => {
    render(<Harness filter={{ from: 'alice' }} initial={DEFAULT_SORTING} />)
    expect(button()).toBeDisabled()
    expect(button()).toHaveAttribute('title', 'Enter a search term to sort by relevance')
  })

  it('switches between relevance and date order', async () => {
    render(<Harness filter={{ subject: 'invoice' }} initial={DEFAULT_SORTING} />)
    expect(button()).toHaveAttribute('aria-pressed', 'false')

    await userEvent.click(button())
    expect(button()).toHaveAttribute('aria-pressed', 'true')
    expect(sorting()).toEqual([])

    await userEvent.click(button())
    expect(button()).toHaveAttribute('aria-pressed', 'false')
    expect(sorting()).toEqual(DEFAULT_SORTING)
  })

  it('falls back to date order once the text is cleared', () => {
    const { rerender } = render(<Harness filter={{ text: 'invoice' }} initial={[]} />)
    expect(button()).toHaveAttribute('aria-pressed', 'true')

    rerender(<Harness filter={{}} initial={[]} />)
    expect(sorting()).toEqual(DEFAULT_SORTING)
    expect(button()).toHaveAttribute('aria-pressed', 'false')
  })
})

describe('sorting and search parameters', () => {
  it('maps an empty sorting to relevance and columns to their sort', () => {
    expect(sortParams([])).toEqual({ sortBy: 'RELEVANCE' })
    expect(sortParams([{ id: 'size', desc: false }])).toEqual({ sortBy: 'SIZE', sortOrder: 'asc' })
    expect(sortParams(DEFAULT_SORTING)).toEqual({ sortBy: 'DATE', sortOrder: 'desc' })
  })

  it('restores the sort from the URL', () => {
    expect(initialSorting('RELEVANCE', 'desc')).toEqual([])
    expect(initialSorting('SIZE', 'asc')).toEqual([{ id: 'size', desc: false }])
    expect(initialSorting('DATE', 'desc')).toEqual(DEFAULT_SORTING)
  })
})
