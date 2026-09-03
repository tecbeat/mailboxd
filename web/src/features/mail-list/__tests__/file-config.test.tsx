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

import { describe, it, expect } from 'vitest'
import { isValidElement } from 'react'
import { getFileConfig } from '../file-config'

describe('getFileConfig', () => {
  it.each([
    ['application/pdf', 'red'],
    ['image/png', 'purple'],
    ['audio/mpeg', 'amber'],
    ['video/mp4', 'pink'],
    ['application/vnd.ms-excel', 'green'],
    ['text/csv', 'green'],
    ['application/zip', 'gray'],
    ['text/plain', 'sky'],
    ['application/json', 'sky'],
    ['application/octet-stream', 'muted'],
  ])('maps %s to the %s palette with a rendered icon', (mime, palette) => {
    const { icon, color } = getFileConfig(mime)
    expect(color).toContain(palette)
    expect(isValidElement(icon)).toBe(true)
  })

  it('matches case-insensitively', () => {
    expect(getFileConfig('APPLICATION/PDF').color).toContain('red')
  })
})
