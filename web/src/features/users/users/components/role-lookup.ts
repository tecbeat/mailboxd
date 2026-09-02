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
