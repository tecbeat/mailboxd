//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.

import { UserRole } from '@/api/users/api';

/** Index roles by id so a row resolves its roles in O(1) instead of scanning the list per row. */
export function rolesById(roles: UserRole[]): Map<number, UserRole> {
  return new Map(roles.map((role) => [role.id, role]));
}

/** Resolve role ids to roles in order, dropping ids that have no matching role. */
export function resolveRoles(ids: number[], byId: Map<number, UserRole>): UserRole[] {
  return ids
    .map((id) => byId.get(id))
    .filter((role): role is UserRole => role !== undefined);
}
