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

import { useCurrentUser } from '@/hooks/use-current-user'
import { Plus } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { get_user_tokens } from '@/api/users/api'
import { Spinner } from '@/components/ui/spinner'
import Logo from '@/assets/logo.svg'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { PageHeader } from '@/components/layout/page-header'
import { ScrollArea } from '@/components/ui/scroll-area'
import { EmptyState } from '@/components/ui/empty-state'
import { TokenCardList, TokensActionDialog } from '@/features/api-tokens'
import { useTranslation } from 'react-i18next'

export function APITokens() {
  const { t } = useTranslation()
  const { data: user, isLoading, error } = useCurrentUser()
  const [addOpen, setAddOpen] = useState(false)

  const { data: tokens = [], isLoading: tokensLoading } = useQuery({
    queryKey: ['user-tokens', user?.id],
    queryFn: () => get_user_tokens(user!.id),
    enabled: !!user?.id,
  })

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Spinner />
      </div>
    )
  }

  if (error || !user) {
    return (
      <div className="p-6 text-red-600">
        {t('apiTokens.page.loadError')}
      </div>
    )
  }

  return (
    <div className="w-full">
      <PageHeader
        className="mb-4"
        title={t('apiTokens.page.title', 'API Tokens')}
        description={t('apiTokens.page.pageDescription', 'Personal API tokens for programmatic access on your behalf.')}
        actions={
          <Button onClick={() => setAddOpen(true)}>
            <span>{t('apiTokens.page.addBtn')}</span>
            <Plus size={18} className="ml-2" />
          </Button>
        }
      />
      {tokensLoading ? (
        <div className="flex justify-center items-center h-64">
          <Spinner />
        </div>
      ) : tokens.length === 0 ? (
        <EmptyState
          className="mt-4"
          icon={
            <img
              src={Logo}
              className="max-h-[100px] w-auto opacity-20 saturate-0 transition-all duration-300 hover:opacity-100 hover:saturate-100 object-contain"
              alt="mailboxd icon"
            />
          }
          title={t('apiTokens.page.emptyTitle')}
          description={t('apiTokens.page.emptyDescription')}
        />
      ) : (
        <ScrollArea className="h-[calc(100vh-16rem)] w-full pr-4 -mr-4 py-1">
          <TokenCardList tokens={tokens} userId={user.id} />
        </ScrollArea>
      )}

      <TokensActionDialog
        key="api-token-add"
        open={addOpen}
        userId={user.id}
        onOpenChange={setAddOpen}
      />
    </div>
  )
}