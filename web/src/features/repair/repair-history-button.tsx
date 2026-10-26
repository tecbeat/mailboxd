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

import { useState } from 'react'
import { History } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { FilterLabel } from '@/features/mail-list/filter-bar'
import { filterSegment } from '@/features/mail-list/filter-bar-styles'
import { useCurrentUser } from '@/hooks/use-current-user'
import { REPAIR_PERMISSIONS } from './permissions'
import { RepairHistoryDrawer } from './repair-history-drawer'

/**
 * Filter-bar segment that opens the repair history. Shown when the user may
 * repair messages in at least one account.
 */
export function RepairHistoryButton() {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const { require_any_permission } = useCurrentUser()

  if (!require_any_permission(REPAIR_PERMISSIONS)) return null

  return (
    <>
      <Button
        variant='ghost'
        className={filterSegment()}
        title={t('repair_message.history')}
        onClick={() => setOpen(true)}
      >
        <History className='h-4 w-4' />
        <FilterLabel>{t('repair_message.history')}</FilterLabel>
      </Button>
      <RepairHistoryDrawer open={open} onOpenChange={setOpen} />
    </>
  )
}
