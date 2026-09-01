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

import { formatBytes, formatTimestamp } from '../utils';

// Characterization tests: these pin the behaviour of the single, shared
// implementations that #13 unified onto (formatBytes previously had three
// divergent copies; formatTimestamp had a duplicate in mail-message-view).

describe('formatBytes', () => {
  it('renders raw bytes below 1 KiB with a "B" unit and no decimals', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1023)).toBe('1023 B');
  });

  it('renders KiB / MiB / GiB with two decimals and a space', () => {
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1536)).toBe('1.50 KB');
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB');
    expect(formatBytes(1024 * 1024 * 1.5)).toBe('1.50 MB');
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.00 GB');
    expect(formatBytes(1024 * 1024 * 1024 * 2.5)).toBe('2.50 GB');
  });

  it('switches unit exactly at each 1024 boundary', () => {
    expect(formatBytes(1024 * 1024 - 1)).toMatch(/ KB$/);
    expect(formatBytes(1024 * 1024)).toMatch(/ MB$/);
  });
});

describe('formatTimestamp', () => {
  it('formats as ISO-8601 local date-time with a timezone offset', () => {
    expect(formatTimestamp(0)).toMatch(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[+-]\d{2}:\d{2}$/,
    );
  });

  it('zero-pads and reproduces the local date-time fields (timezone-independent)', () => {
    // Constructed in local time, so the local fields the formatter reads back
    // must match regardless of the machine/CI timezone.
    const local = new Date(2023, 0, 5, 3, 7, 9); // 2023-01-05 03:07:09 local
    expect(formatTimestamp(local.getTime())).toMatch(
      /^2023-01-05T03:07:09[+-]\d{2}:\d{2}$/,
    );
  });

  it('round-trips: parsing the output yields the original instant (to the second)', () => {
    const ms = Date.UTC(2024, 5, 15, 12, 30, 45); // arbitrary instant
    expect(new Date(formatTimestamp(ms)).getTime()).toBe(ms);
  });
});
