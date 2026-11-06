//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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

import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { defaultColumnSettings, type ColumnSetting } from '@/lib/table-columns'
import { renderWithProviders } from '@/test/utils'

import { ColumnSettings } from '../column-settings'
import { mockListLayout, rowOrder } from '@/test/list-layout'

function Harness({ initial, onChange }: { initial: ColumnSetting[]; onChange?: (value: ColumnSetting[]) => void }) {
  const [value, setValue] = useState(initial)
  return (
    <ColumnSettings
      table='accounts'
      value={value}
      onChange={(next) => {
        setValue(next)
        onChange?.(next)
      }}
    />
  )
}

describe('ColumnSettings', () => {
  let restoreLayout: () => void
  beforeEach(() => {
    restoreLayout = mockListLayout()
  })
  afterEach(() => restoreLayout())

  it('lists every movable column in the given order', () => {
    const initial = defaultColumnSettings('accounts').reverse()
    renderWithProviders(<Harness initial={initial} />)
    expect(rowOrder()).toEqual(initial.map((setting) => setting.id))
    expect(screen.getAllByRole('button', { name: /^Move / })).toHaveLength(11)
  })

  it('switches an optional column off and on', async () => {
    const onChange = vi.fn()
    renderWithProviders(<Harness initial={defaultColumnSettings('accounts')} onChange={onChange} />)

    await userEvent.click(screen.getByRole('switch', { name: 'Show ID' }))
    expect(screen.getByRole('switch', { name: 'Show ID' })).not.toBeChecked()
    expect(onChange).toHaveBeenLastCalledWith(
      expect.arrayContaining([{ id: 'id', visible: false }, { id: 'account_name', visible: true }]),
    )

    await userEvent.click(screen.getByRole('switch', { name: 'Show ID' }))
    expect(screen.getByRole('switch', { name: 'Show ID' })).toBeChecked()
  })

  it('locks the switch of columns that are always shown', () => {
    const initial = defaultColumnSettings('accounts').map((s) => (s.id === 'email' ? { ...s, visible: false } : s))
    renderWithProviders(<Harness initial={initial} />)
    for (const name of ['Email', 'Enabled', 'Auth']) {
      const toggle = screen.getByRole('switch', { name: `Show ${name}` })
      expect(toggle).toBeChecked()
      expect(toggle).toBeDisabled()
      expect(within(toggle.closest('li')!).getByText('Always shown')).toBeInTheDocument()
    }
  })

  it('moves a column with the keyboard', async () => {
    const onChange = vi.fn()
    renderWithProviders(<Harness initial={defaultColumnSettings('accounts')} onChange={onChange} />)

    screen.getByRole('button', { name: 'Move ID' }).focus()
    await userEvent.keyboard('[Space]')
    await userEvent.keyboard('[ArrowDown]')
    await userEvent.keyboard('[ArrowDown]')
    await userEvent.keyboard('[Space]')

    expect(rowOrder().slice(0, 3)).toEqual(['account_name', 'email', 'id'])
    expect(onChange).toHaveBeenCalledTimes(1)
  })

  it('announces the pickup, each move and the drop to screen readers', async () => {
    renderWithProviders(<Harness initial={defaultColumnSettings('accounts')} />)
    const live = () => document.querySelector('[aria-live]')?.textContent

    screen.getByRole('button', { name: 'Move ID' }).focus()
    await userEvent.keyboard('[Space]')
    expect(live()).toBe('Picked up ID at position 1.')
    await userEvent.keyboard('[ArrowDown]')
    expect(live()).toBe('ID moved to position 2.')
    await userEvent.keyboard('[Space]')
    expect(live()).toBe('ID dropped at position 2.')
  })

  it('leaves the order alone when a keyboard move is cancelled', async () => {
    const onChange = vi.fn()
    renderWithProviders(<Harness initial={defaultColumnSettings('accounts')} onChange={onChange} />)

    screen.getByRole('button', { name: 'Move ID' }).focus()
    await userEvent.keyboard('[Space]')
    await userEvent.keyboard('[ArrowDown]')
    await userEvent.keyboard('[Escape]')

    expect(rowOrder()[0]).toBe('id')
    expect(onChange).not.toHaveBeenCalled()
  })

  it('restores the default order and shows every column', async () => {
    const initial = defaultColumnSettings('accounts')
      .reverse()
      .map((s) => ({ ...s, visible: false }))
    renderWithProviders(<Harness initial={initial} />)

    await userEvent.click(screen.getByRole('button', { name: 'Restore default columns' }))

    expect(rowOrder()).toEqual(defaultColumnSettings('accounts').map((s) => s.id))
    expect(screen.getByRole('switch', { name: 'Show ID' })).toBeChecked()
  })
})
