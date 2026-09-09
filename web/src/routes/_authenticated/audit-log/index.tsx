//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
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


import { createFileRoute } from '@tanstack/react-router'
import AuditLog from '@/features/audit-log'
import { getPersistedPageSize } from '@/lib/data-table'
import { z } from 'zod'

const searchSchema = z.object({
  page: z.number().catch(1),
  pageSize: z.number().optional(),
})

export const Route = createFileRoute('/_authenticated/audit-log/')({
  component: AuditLog,
  validateSearch: (search) => {
    const result = searchSchema.parse(search)
    return {
      ...result,
      page: result.page ?? 1,
      pageSize: result.pageSize ?? getPersistedPageSize('audit-log', 30),
    }
  },
})
