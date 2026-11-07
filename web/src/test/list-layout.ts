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

import { screen } from '@testing-library/react'

const ROW_HEIGHT = 40

/**
 * jsdom has no layout, so dnd-kit cannot tell where the rows of a sortable
 * list are. Give every row of the column list a 40px slot by its position
 * in the list; returns a function that restores the original measurement.
 */
export function mockListLayout(): () => void {
  const original = Element.prototype.getBoundingClientRect
  Element.prototype.getBoundingClientRect = function (this: Element) {
    const row = this.closest('li[data-column-id]')
    if (!row || row !== this || !row.parentElement) return original.call(this)
    const index = Array.from(row.parentElement.children).indexOf(row)
    const top = index * ROW_HEIGHT
    return {
      x: 0,
      y: top,
      top,
      left: 0,
      right: 400,
      bottom: top + ROW_HEIGHT,
      width: 400,
      height: ROW_HEIGHT,
      toJSON: () => ({}),
    } as DOMRect
  }
  return () => {
    Element.prototype.getBoundingClientRect = original
  }
}

/** The column ids of the column list, top to bottom. */
export function rowOrder(): string[] {
  return screen
    .getAllByRole('listitem')
    .map((row) => row.getAttribute('data-column-id'))
    .filter((id): id is string => id !== null)
}
