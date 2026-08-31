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
