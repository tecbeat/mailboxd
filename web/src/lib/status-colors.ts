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

export type StatusKind = 'success' | 'warning' | 'info' | 'error' | 'neutral';

export const statusBadgeClass: Record<StatusKind, string> = {
  success: 'bg-green-500/10 text-green-600 border-green-500/20',
  warning: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  info: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  error: 'bg-red-500/10 text-red-600 border-red-500/20',
  neutral: 'bg-muted text-muted-foreground border-transparent',
};

export const statusTextClass: Record<StatusKind, string> = {
  success: 'text-green-600',
  warning: 'text-amber-600',
  info: 'text-blue-600',
  error: 'text-destructive',
  neutral: 'text-muted-foreground',
};

export type TriggerKind = 'scheduled' | 'manual';

export const triggerBadgeClass: Record<TriggerKind, string> = {
  scheduled: 'bg-purple-500/10 text-purple-600',
  manual: 'bg-orange-500/10 text-orange-600',
};
