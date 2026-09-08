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

import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Spinner } from '../spinner';

describe('Spinner', () => {
  it('renders the single block-loader size and spin animation by default', () => {
    const { container } = render(<Spinner />);
    const svg = container.querySelector('svg') as SVGElement;

    expect(svg.getAttribute('class')).toContain('h-8');
    expect(svg.getAttribute('class')).toContain('w-8');
    expect(svg.getAttribute('class')).toContain('animate-spin');
    expect(svg.getAttribute('class')).toContain('text-muted-foreground');
  });

  it('lets callers override the size via className', () => {
    const { container } = render(<Spinner className="h-5 w-5" />);
    const svg = container.querySelector('svg') as SVGElement;

    expect(svg.getAttribute('class')).toContain('h-5');
    expect(svg.getAttribute('class')).toContain('w-5');
    expect(svg.getAttribute('class')).not.toContain('h-8');
    expect(svg.getAttribute('class')).toContain('animate-spin');
  });

  it('passes through aria attributes for accessibility', () => {
    const { container } = render(<Spinner aria-label="Loading" role="status" />);
    const svg = container.querySelector('svg') as SVGElement;

    expect(svg.getAttribute('aria-label')).toBe('Loading');
    expect(svg.getAttribute('role')).toBe('status');
  });
});
