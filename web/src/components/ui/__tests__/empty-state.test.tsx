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

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { EmptyState } from '../empty-state';

describe('EmptyState', () => {
  it('renders the title and optional description', () => {
    render(<EmptyState title="No items" description="Nothing to show yet." />);

    expect(screen.getByRole('heading', { name: 'No items' })).toBeInTheDocument();
    expect(screen.getByText('Nothing to show yet.')).toBeInTheDocument();
  });

  it('renders an action only when one is provided', () => {
    const { rerender } = render(<EmptyState title="Empty" />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    rerender(
      <EmptyState title="Empty" action={<button type="button">Create</button>} />,
    );
    expect(screen.getByRole('button', { name: 'Create' })).toBeInTheDocument();
  });

  it('uses a responsive full-width box with a single baseline height and no fixed pixel width', () => {
    const { container } = render(<EmptyState title="Empty" />);
    const box = container.firstElementChild as HTMLElement;

    expect(box.className).toContain('w-full');
    expect(box.className).toContain('min-h-[400px]');
    expect(box.className).toContain('border-dashed');
    expect(box.className).not.toMatch(/\bw-\[\d+px\]/);
  });
});
