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


import { get_user_tokens, User } from '@/api/users/api'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { useQuery } from '@tanstack/react-query'
import { TokenCardList, TokensActionDialog } from '@/features/api-tokens'
import { Skeleton } from '@/components/ui/skeleton'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { useState } from 'react'
import { Plus } from 'lucide-react'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useTranslation } from 'react-i18next'

interface Props {
  currentRow?: User
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function UserApiTokensDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const [addOpen, setAddOpen] = useState(false)

  const { data: tokens = [], isLoading: tokensLoading } = useQuery({
    queryKey: ['user-tokens', currentRow?.id],
    queryFn: () => get_user_tokens(currentRow!.id),
    enabled: !!currentRow?.id,
  })

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => onOpenChange(state)}
    >
      <DialogContent className='md:max-w-2xl'>
        <DialogHeader>
          <DialogTitle>{t('users.tokens_action.dialog.title')}</DialogTitle>
          <DialogDescription>
            {t('users.tokens_action.dialog.description')}
          </DialogDescription>
        </DialogHeader>

        {tokensLoading ? (
          <div className="flex flex-col gap-4 mt-4">
            <Skeleton className="h-16 w-full rounded-lg" />
            <Skeleton className="h-16 w-full rounded-lg" />
            <Skeleton className="h-16 w-full rounded-lg" />
          </div>
        ) : tokens.length === 0 ? (
          <EmptyState
            className="mt-4"
            title={t('users.tokens_action.empty.title')}
            description={t('users.tokens_action.empty.description')}
            action={
              <Button onClick={() => setAddOpen(true)}>
                <Plus className="mr-2 h-4 w-4" /><span>{t('users.tokens_action.buttons.add')}</span>
              </Button>
            }
          />
        ) : (
          <>
            <div className="flex justify-end mb-4">
              <Button onClick={() => setAddOpen(true)}>
                <Plus className="mr-2 h-4 w-4" /><span>{t('users.tokens_action.buttons.add')}</span>
              </Button>
            </div>
            <ScrollArea className='h-[32rem] w-full pr-4 -mr-4 py-1'>
              <TokenCardList tokens={tokens} userId={currentRow!.id} />
            </ScrollArea>
          </>
        )}

        <TokensActionDialog
          key='api-token-add'
          open={addOpen}
          userId={currentRow!.id}
          onOpenChange={setAddOpen}
        />
      </DialogContent>
    </Dialog>
  )
}
