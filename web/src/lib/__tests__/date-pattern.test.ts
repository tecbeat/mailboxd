//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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

import { format } from 'date-fns'
import { describe, expect, it } from 'vitest'

import { isValidDatePattern, toDateFnsPattern } from '../date-pattern'

const SAMPLE = new Date(2026, 0, 5, 14, 7, 9)

function render(pattern: string): string {
  const converted = toDateFnsPattern(pattern)
  if (converted === null) throw new Error(`invalid: ${pattern}`)
  return format(SAMPLE, converted)
}

describe('toDateFnsPattern', () => {
  it.each([
    ['DD.MM.YYYY', '05.01.2026'],
    ['MM.DD.YYYY', '01.05.2026'],
    ['MM.YY', '01.26'],
    ['YYYY-MM-DD HH:mm', '2026-01-05 14:07'],
    ['D.M.YY H:mm:ss', '5.1.26 14:07:09'],
    ['hh:mm A', '02:07 PM'],
    ['dddd, Do MMMM YYYY', 'Monday, 5th January 2026'],
    ['ddd DD MMM', 'Mon 05 Jan'],
  ])('renders %s as %s', (pattern, expected) => {
    expect(render(pattern)).toBe(expected)
  })

  it('keeps text in brackets and quotes literally', () => {
    expect(render('[Week] W, YYYY')).toBe('Week 2, 2026')
    expect(render("[o'clock] HH")).toBe("o'clock 14")
    expect(render("DD'MM")).toBe("05'01")
  })

  it.each(['', '   ', '...', 'JJ.MM', 'DD.MM.YYYX', '[DD.MM', 'YYYY-MM-DD T', 'x'.repeat(65)])(
    'rejects %j',
    (pattern) => {
      expect(toDateFnsPattern(pattern)).toBeNull()
    },
  )
})

describe('isValidDatePattern', () => {
  it('accepts an empty pattern as the language default', () => {
    expect(isValidDatePattern('')).toBe(true)
    expect(isValidDatePattern('  ')).toBe(true)
  })

  it('accepts valid and rejects invalid patterns', () => {
    expect(isValidDatePattern('MM.YY')).toBe(true)
    expect(isValidDatePattern('MM.JJ')).toBe(false)
  })
})
