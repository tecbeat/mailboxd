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


// A date pattern is written with Moment.js / Day.js format tokens (the
// notation most users know, e.g. "DD.MM.YYYY"); date-fns uses Unicode tokens,
// where "DD" and "YYYY" mean something else. Text in [brackets] is literal.

const TOKENS: Record<string, string> = {
  YYYY: 'yyyy',
  YY: 'yy',
  Q: 'Q',
  MMMM: 'MMMM',
  MMM: 'MMM',
  MM: 'MM',
  M: 'M',
  Do: 'do',
  DD: 'dd',
  D: 'd',
  dddd: 'EEEE',
  ddd: 'EEE',
  WW: 'II',
  W: 'I',
  ww: 'ww',
  w: 'w',
  HH: 'HH',
  H: 'H',
  hh: 'hh',
  h: 'h',
  mm: 'mm',
  m: 'm',
  ss: 'ss',
  s: 's',
  A: 'a',
  a: 'aaa',
  ZZ: 'xx',
  Z: 'xxx',
}

const BY_LENGTH = Object.keys(TOKENS).sort((a, b) => b.length - a.length)

const MAX_LENGTH = 64

function quote(text: string): string {
  return text ? `'${text.replace(/'/g, "''")}'` : ''
}

/**
 * The date-fns pattern for a Moment.js-style `pattern`, or `null` when the
 * pattern is invalid: an unknown letter, an unclosed bracket, or no token.
 */
export function toDateFnsPattern(pattern: string): string | null {
  if (pattern.length > MAX_LENGTH) return null
  let out = ''
  let tokens = 0
  let i = 0
  while (i < pattern.length) {
    const char = pattern[i]
    if (char === '[') {
      const end = pattern.indexOf(']', i + 1)
      if (end === -1) return null
      out += quote(pattern.slice(i + 1, end))
      i = end + 1
    } else if (/[A-Za-z]/.test(char)) {
      const token = BY_LENGTH.find((candidate) => pattern.startsWith(candidate, i))
      if (!token) return null
      out += TOKENS[token]
      tokens += 1
      i += token.length
    } else {
      out += char === "'" ? "''" : char
      i += 1
    }
  }
  return tokens > 0 ? out : null
}

/** Whether `pattern` is empty (the language default) or a valid pattern. */
export function isValidDatePattern(pattern: string): boolean {
  return pattern.trim() === '' || toDateFnsPattern(pattern) !== null
}
