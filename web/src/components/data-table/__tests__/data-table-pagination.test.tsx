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

import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import userEvent from '@testing-library/user-event'
import { render, screen } from '@/test/test-utils'
import { DataTablePagination } from '../data-table-pagination'

const STORAGE_KEY = 'pagination_test'

function renderControlled(
  overrides: Partial<Parameters<typeof DataTablePagination>[0]> = {},
) {
  const onPageSizeChange = vi.fn()
  const props = {
    storageKey: STORAGE_KEY,
    pageIndex: 2,
    pageCount: 10,
    pageSize: 30,
    canPreviousPage: true,
    canNextPage: true,
    onFirst: vi.fn(),
    onPrevious: vi.fn(),
    onNext: vi.fn(),
    onLast: vi.fn(),
    onPageSizeChange,
    ...overrides,
  }
  render(<DataTablePagination {...props} />)
  return { onPageSizeChange }
}

describe('DataTablePagination (controlled mode)', () => {
  beforeAll(() => {
    // Radix Select relies on these DOM APIs that jsdom does not implement.
    window.HTMLElement.prototype.scrollIntoView = vi.fn()
    window.HTMLElement.prototype.hasPointerCapture = vi.fn(() => false)
    window.HTMLElement.prototype.releasePointerCapture = vi.fn()
  })

  beforeEach(() => {
    localStorage.clear()
  })

  it('shows the human page number and total from pageIndex/pageCount', () => {
    renderControlled({ pageIndex: 2, pageCount: 10 })
    expect(screen.getByText('Page 3 of 10')).toBeInTheDocument()
  })

  it('disables first and previous when there is no previous page', () => {
    renderControlled({ canPreviousPage: false, canNextPage: true })
    expect(screen.getByRole('button', { name: 'Go to first page' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Next page' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Go to last page' })).toBeEnabled()
  })

  it('disables next and last when there is no next page', () => {
    renderControlled({ canPreviousPage: true, canNextPage: false })
    expect(screen.getByRole('button', { name: 'Go to first page' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Go to last page' })).toBeDisabled()
  })

  it('persists the chosen page size and notifies the parent on change', async () => {
    const user = userEvent.setup()
    const { onPageSizeChange } = renderControlled({ pageSize: 30 })

    await user.click(screen.getByRole('combobox'))
    await user.click(await screen.findByRole('option', { name: '50' }))

    expect(onPageSizeChange).toHaveBeenCalledWith(50)
    expect(localStorage.getItem(`mailboxd_${STORAGE_KEY}_page_size`)).toBe('50')
  })
})
