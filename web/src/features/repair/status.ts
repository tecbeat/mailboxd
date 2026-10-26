//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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

import type { TFunction } from 'i18next'
import type { BlobRepairStatus } from '@/api/repair/api'

type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline'

export function repairStatusLabel(t: TFunction, status: BlobRepairStatus): string {
  const labels: Record<BlobRepairStatus, string> = {
    complete: t('repair_message.statusComplete'),
    repaired: t('repair_message.statusRepaired'),
    partial: t('repair_message.statusPartial'),
    failed: t('repair_message.statusFailed'),
  }
  return labels[status] ?? status
}

const VARIANTS: Record<BlobRepairStatus, BadgeVariant> = {
  complete: 'secondary',
  repaired: 'default',
  partial: 'outline',
  failed: 'destructive',
}

export function repairStatusVariant(status: BlobRepairStatus): BadgeVariant {
  return VARIANTS[status] ?? 'outline'
}
