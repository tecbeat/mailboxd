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

import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'

// A modal Dialog disables pointer events on <body> and traps focus. A Popover
// opened from inside it must still take mouse and keyboard input. That only
// holds while all Radix primitives share one dismissable-layer/focus-scope
// copy; with mixed versions the popover is unclickable and loses focus (#10).
function PopoverInDialog({ onPick }: { onPick: () => void }) {
  return (
    <Dialog open>
      <DialogContent>
        <DialogTitle>Authorize</DialogTitle>
        <DialogDescription>Pick an account</DialogDescription>
        <Popover>
          <PopoverTrigger>Select an account</PopoverTrigger>
          <PopoverContent>
            <input aria-label='Search' autoFocus />
            <button type='button' onClick={onPick}>
              user1@example.test
            </button>
          </PopoverContent>
        </Popover>
      </DialogContent>
    </Dialog>
  )
}

describe('Popover inside a modal Dialog', () => {
  it('accepts mouse clicks', async () => {
    const user = userEvent.setup()
    const onPick = vi.fn()
    render(<PopoverInDialog onPick={onPick} />)

    await user.click(screen.getByRole('button', { name: 'Select an account' }))
    await user.click(await screen.findByRole('button', { name: 'user1@example.test' }))

    expect(onPick).toHaveBeenCalledOnce()
  })

  it('keeps keyboard focus inside the popover', async () => {
    const user = userEvent.setup()
    render(<PopoverInDialog onPick={() => {}} />)

    await user.click(screen.getByRole('button', { name: 'Select an account' }))
    const search = await screen.findByRole('textbox', { name: 'Search' })
    await user.keyboard('abc')

    expect(search).toHaveFocus()
    expect(search).toHaveValue('abc')
  })
})
