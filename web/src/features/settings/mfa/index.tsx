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
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Info, ShieldPlus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  enroll_mfa,
  get_mfa_status,
  type MfaEnrolment,
  type MfaStatus,
} from '@/api/users/api'
import { toast } from '@/hooks/use-toast'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Spinner } from '@/components/ui/spinner'
import { PageHeader } from '@/components/layout/page-header'
import { EnabledCard } from './enabled-card'
import { EnrolmentCard } from './enrolment-card'
import { describeMfaError } from './mfa-error'
import { RecoveryCodesCard } from './recovery-codes-card'

export function SettingsMfa() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [enrolment, setEnrolment] = useState<MfaEnrolment | null>(null)
  const [recoveryCodes, setRecoveryCodes] = useState<string[] | null>(null)

  const { data: status, isError } = useQuery({
    queryKey: ['mfa-status'],
    queryFn: get_mfa_status,
  })

  const enrolMutation = useMutation({
    mutationFn: enroll_mfa,
    onSuccess: setEnrolment,
    onError: (error) => {
      toast({
        variant: 'destructive',
        title: t('settings.mfa.errors.enrolFailed'),
        description: describeMfaError(error, t),
      })
    },
  })

  function renderContent(status: MfaStatus) {
    if (recoveryCodes) {
      return (
        <RecoveryCodesCard
          codes={recoveryCodes}
          onDone={() => setRecoveryCodes(null)}
        />
      )
    }
    if (enrolment) {
      return (
        <EnrolmentCard
          enrolment={enrolment}
          onConfirmed={(codes) => {
            setEnrolment(null)
            setRecoveryCodes(codes)
            queryClient.invalidateQueries({ queryKey: ['mfa-status'] })
          }}
          onCancel={() => setEnrolment(null)}
        />
      )
    }
    if (status.enabled) {
      return <EnabledCard status={status} />
    }
    if (!status.available) {
      return (
        <Alert>
          <Info className='h-4 w-4' />
          <AlertDescription>{t('settings.mfa.ssoNote')}</AlertDescription>
        </Alert>
      )
    }
    return (
      <Card>
        <CardHeader>
          <CardTitle>{t('settings.mfa.disabled.title')}</CardTitle>
          <CardDescription>
            {t('settings.mfa.disabled.description')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            disabled={enrolMutation.isPending}
            onClick={() => enrolMutation.mutate()}
          >
            <ShieldPlus className='mr-2 h-4 w-4' />
            {t('settings.mfa.disabled.enable')}
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <div className='w-full'>
      <PageHeader
        className='mb-4'
        title={t('settings.mfa.title')}
        description={t('settings.mfa.description')}
      />
      {status ? (
        renderContent(status)
      ) : isError ? (
        <p className='text-sm text-destructive'>
          {t('settings.mfa.loadError')}
        </p>
      ) : (
        <Spinner />
      )}
    </div>
  )
}
