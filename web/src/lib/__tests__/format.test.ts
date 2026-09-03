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
import { de, enUS } from 'date-fns/locale';

import { formatBytes, formatDateTime, formatRelativeTime } from '../utils';

// Characterization tests for the single, shared formatting helpers.
// formatBytes was unified by #13; formatDateTime and formatRelativeTime were
// unified by #31 and localise absolute and relative timestamps consistently.
// The date-time tests pass an explicit locale so the assertions stay stable
// regardless of the ambient i18n language.

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

describe('formatDateTime', () => {
  // Local time so the reproduced fields are timezone-independent.
  const reference = new Date(2023, 0, 5, 14, 30);

  it('renders a localized date and short time (no seconds)', () => {
    const enOutput = formatDateTime(reference, enUS);
    expect(enOutput).not.toBe('');
    expect(enOutput).toContain('2023');
    // The short time carries no seconds component.
    expect(enOutput).not.toMatch(/:\d{2}:\d{2}/);
  });

  it('localizes differently for en-US and de', () => {
    const enOutput = formatDateTime(reference, enUS);
    const deOutput = formatDateTime(reference, de);
    expect(deOutput).not.toBe('');
    expect(deOutput).toContain('2023');
    expect(enOutput).not.toBe(deOutput);
  });

  it('accepts a numeric epoch and an ISO string, agreeing with a Date', () => {
    const expected = formatDateTime(reference, enUS);
    expect(formatDateTime(reference.getTime(), enUS)).toBe(expected);
    expect(formatDateTime(reference.toISOString(), enUS)).toBe(expected);
  });

  it('returns an empty string for an invalid date', () => {
    expect(formatDateTime(NaN, enUS)).toBe('');
    expect(formatDateTime('not-a-date', enUS)).toBe('');
  });
});

describe('formatRelativeTime', () => {
  const twoHoursAgo = new Date(Date.now() - 2 * 60 * 60 * 1000);

  it('renders a relative phrase with an English suffix', () => {
    const output = formatRelativeTime(twoHoursAgo, enUS);
    expect(output).not.toBe('');
    expect(output).toContain('ago');
  });

  it('localizes the relative suffix for de', () => {
    const enOutput = formatRelativeTime(twoHoursAgo, enUS);
    const deOutput = formatRelativeTime(twoHoursAgo, de);
    expect(deOutput).not.toBe('');
    expect(deOutput).toContain('vor');
    expect(enOutput).not.toBe(deOutput);
  });

  it('returns an empty string for an invalid date', () => {
    expect(formatRelativeTime(NaN, enUS)).toBe('');
  });
});
