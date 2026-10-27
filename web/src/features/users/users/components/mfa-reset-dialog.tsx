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
import { AxiosError } from 'axios'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { reset_user_mfa, User } from '@/api/users/api'
import { toast } from '@/hooks/use-toast'
import { ConfirmDialog } from '@/components/confirm-dialog'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: User
}

export function UserMfaResetDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const resetMutation = useMutation({
    mutationFn: () => reset_user_mfa(currentRow.id),
    onSuccess: () => {
      toast({
        title: t('users.actions.reset_mfa.success_title'),
        description: t('users.actions.reset_mfa.success_desc', {
          name: currentRow.username,
        }),
      })
      queryClient.invalidateQueries({ queryKey: ['user-list'] })
      onOpenChange(false)
    },
    onError: (error: AxiosError) => {
      toast({
        variant: 'destructive',
        title: t('users.actions.reset_mfa.failed_title'),
        description:
          (error.response?.data as { message?: string } | undefined)?.message ||
          error.message,
      })
    },
  })

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={() => resetMutation.mutate()}
      isLoading={resetMutation.isPending}
      destructive
      title={t('users.actions.reset_mfa.title')}
      desc={t('users.actions.reset_mfa.confirm_msg', {
        name: currentRow.username,
        id: currentRow.id,
      })}
      confirmText={t('users.actions.reset_mfa.button_confirm')}
    />
  )
}
