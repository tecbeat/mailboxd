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

import { describe, expect, it } from 'vitest';

import { countSelectedMailboxes } from '../mailbox-selection';

describe('countSelectedMailboxes', () => {
  const mailboxes = [{ id: 1 }, { id: 2 }, { id: 3 }];

  it('counts how many mailboxes are selected', () => {
    expect(countSelectedMailboxes(mailboxes, new Set([1, 3]))).toBe(2);
  });

  it('ignores selected ids that are not in the mailbox list', () => {
    expect(countSelectedMailboxes(mailboxes, new Set([3, 99]))).toBe(1);
  });

  it('returns 0 for missing data or an empty selection', () => {
    expect(countSelectedMailboxes(undefined, new Set([1]))).toBe(0);
    expect(countSelectedMailboxes(mailboxes, new Set())).toBe(0);
  });
});
