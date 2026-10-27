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

import { describe, expect, it } from 'vitest'

import en from '../en.json'

const sources = import.meta.glob(['/src/**/*.{ts,tsx}', '!/src/**/__tests__/**', '!/src/**/*.test.*'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

// Literal keys only: t('a.b') or t("a.b"), optionally with options.
// Keys built at runtime (template literals) are not checked.
const KEY = /\bt\(\s*['"]([A-Za-z0-9_]+(?:\.[A-Za-z0-9_-]+)+)['"]/g

function exists(key: string): boolean {
  let node: unknown = en
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null || !(part in node)) return false
    node = (node as Record<string, unknown>)[part]
  }
  return typeof node === 'string'
}

/** A key with plural forms (key_one, key_other) is looked up by its base. */
function resolves(key: string): boolean {
  return exists(key) || exists(`${key}_one`) || exists(`${key}_other`)
}

describe('translation keys', () => {
  it('scans the source files', () => {
    expect(Object.keys(sources).length).toBeGreaterThan(100)
  })

  it('finds every key the code uses in the English locale', () => {
    const missing = Object.entries(sources).flatMap(([file, source]) =>
      [...source.matchAll(KEY)]
        .map((match) => match[1])
        .filter((key) => !resolves(key))
        .map((key) => `${file}: ${key}`),
    )

    expect(missing).toEqual([])
  })
})
