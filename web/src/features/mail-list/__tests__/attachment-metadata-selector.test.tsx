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
import userEvent from '@testing-library/user-event'
import { render, screen } from '@/test/test-utils'
import { MetadataSelectorField } from '../attachment-metadata-selector'
import type { Group } from '@/api/system/api'

const options: Group[] = [
  { key: 'pdf', count: 3 },
  { key: 'png', count: 5 },
]

describe('MetadataSelectorField', () => {
  it('renders the field label', () => {
    render(
      <MetadataSelectorField
        label="Extension"
        options={options}
        isLoading={false}
        onSelect={vi.fn()}
        onReset={vi.fn()}
      />,
    )
    expect(screen.getByText('Extension')).toBeInTheDocument()
  })

  it('shows the selected value', () => {
    render(
      <MetadataSelectorField
        label="Extension"
        value="pdf"
        options={options}
        isLoading={false}
        onSelect={vi.fn()}
        onReset={vi.fn()}
      />,
    )
    expect(screen.getByText('pdf')).toBeInTheDocument()
  })

  it('calls onReset when the clear control is clicked', async () => {
    const onReset = vi.fn()
    const { container } = render(
      <MetadataSelectorField
        label="Extension"
        value="pdf"
        options={options}
        isLoading={false}
        onSelect={vi.fn()}
        onReset={onReset}
      />,
    )
    const clearIcon = container.querySelector('.lucide-x')
    expect(clearIcon).not.toBeNull()
    await userEvent.click(clearIcon!.parentElement as HTMLElement)
    expect(onReset).toHaveBeenCalledTimes(1)
  })
})
