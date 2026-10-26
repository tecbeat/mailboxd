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
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ShieldCheck, ShieldOff } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { disable_mfa, type MfaStatus } from '@/api/users/api'
import { toast } from '@/hooks/use-toast'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { CodeForm } from './code-form'
import { describeMfaError } from './mfa-error'

// Below this many unused recovery codes the user is nudged to renew them.
const LOW_RECOVERY_CODES = 3

export function EnabledCard({ status }: { status: MfaStatus }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [disabling, setDisabling] = useState(false)

  const mutation = useMutation({
    mutationFn: disable_mfa,
    onSuccess: () => {
      toast({ title: t('settings.mfa.enabled.disabledToast') })
      setDisabling(false)
      queryClient.invalidateQueries({ queryKey: ['mfa-status'] })
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle className='flex items-center gap-2'>
          <ShieldCheck className='h-5 w-5 text-primary' />
          {t('settings.mfa.enabled.title')}
        </CardTitle>
        <CardDescription>
          {t('settings.mfa.enabled.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className='space-y-4'>
        <div className='space-y-1 text-sm'>
          <p>
            {t('settings.mfa.enabled.codesLeft', {
              remaining: status.recovery_codes_left,
            })}
          </p>
          {status.recovery_codes_left < LOW_RECOVERY_CODES && (
            <p className='text-destructive'>
              {t('settings.mfa.enabled.codesLow')}
            </p>
          )}
        </div>
        {disabling ? (
          <CodeForm
            id='mfa-disable-code'
            label={t('settings.mfa.enabled.disableCode')}
            submitLabel={t('settings.mfa.enabled.confirmDisable')}
            destructive
            isPending={mutation.isPending}
            error={mutation.error ? describeMfaError(mutation.error, t) : null}
            onSubmit={(code) => mutation.mutate(code)}
            onCancel={() => {
              setDisabling(false)
              mutation.reset()
            }}
          />
        ) : (
          <Button variant='destructive' onClick={() => setDisabling(true)}>
            <ShieldOff className='mr-2 h-4 w-4' />
            {t('settings.mfa.enabled.disable')}
          </Button>
        )}
      </CardContent>
    </Card>
  )
}
