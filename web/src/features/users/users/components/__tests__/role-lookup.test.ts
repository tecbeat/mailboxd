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

import type { UserRole } from '@/api/users/api';

import { resolveRoles, rolesById } from '../role-lookup';

const roles = [
  { id: 1, name: 'admin' },
  { id: 2, name: 'auditor' },
] as unknown as UserRole[];

describe('rolesById', () => {
  it('indexes roles by their id', () => {
    const byId = rolesById(roles);
    expect(byId.get(1)?.name).toBe('admin');
    expect(byId.get(2)?.name).toBe('auditor');
    expect(byId.size).toBe(2);
  });
});

describe('resolveRoles', () => {
  it('resolves ids to roles in order', () => {
    const byId = rolesById(roles);
    expect(resolveRoles([2, 1], byId).map((r) => r.name)).toEqual(['auditor', 'admin']);
  });

  it('drops ids that have no matching role', () => {
    const byId = rolesById(roles);
    expect(resolveRoles([1, 999], byId).map((r) => r.name)).toEqual(['admin']);
  });
});
