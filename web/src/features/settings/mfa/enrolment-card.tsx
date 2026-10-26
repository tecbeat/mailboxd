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
import { useEffect, useState } from 'react'
import { useMutation } from '@tanstack/react-query'
import { Copy } from 'lucide-react'
import * as QRCode from 'qrcode'
import { useTranslation } from 'react-i18next'
import { confirm_mfa, type MfaEnrolment } from '@/api/users/api'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { Spinner } from '@/components/ui/spinner'
import { copyToClipboard } from './clipboard'
import { CodeForm } from './code-form'
import { describeMfaError } from './mfa-error'

interface EnrolmentCardProps {
  enrolment: MfaEnrolment
  onConfirmed: (recoveryCodes: string[]) => void
  onCancel: () => void
}

export function EnrolmentCard({
  enrolment,
  onConfirmed,
  onCancel,
}: EnrolmentCardProps) {
  const { t } = useTranslation()
  // undefined while rendering, null if the QR code could not be generated.
  const [qrCode, setQrCode] = useState<string | null | undefined>(undefined)

  useEffect(() => {
    let active = true
    QRCode.toString(enrolment.otpauth_uri, { type: 'svg', margin: 1 }).then(
      (svg) => {
        if (active) {
          setQrCode(
            `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
          )
        }
      },
      () => {
        if (active) {
          setQrCode(null)
        }
      }
    )
    return () => {
      active = false
    }
  }, [enrolment.otpauth_uri])

  const mutation = useMutation({
    mutationFn: confirm_mfa,
    onSuccess: (data) => onConfirmed(data.recovery_codes),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.mfa.enrol.title')}</CardTitle>
        <CardDescription>{t('settings.mfa.enrol.description')}</CardDescription>
      </CardHeader>
      <CardContent className='space-y-4'>
        <div className='flex h-52 w-52 items-center justify-center rounded-md border bg-white p-2'>
          {qrCode === undefined ? (
            <Spinner />
          ) : qrCode === null ? (
            <p className='text-center text-sm text-muted-foreground'>
              {t('settings.mfa.enrol.qrFailed')}
            </p>
          ) : (
            <img
              src={qrCode}
              alt={t('settings.mfa.enrol.qrAlt')}
              className='h-full w-full'
            />
          )}
        </div>
        <div className='space-y-1'>
          <Label>{t('settings.mfa.enrol.secret')}</Label>
          <div className='flex items-center gap-2'>
            <code className='rounded border bg-muted px-2 py-1 font-mono text-sm break-all'>
              {enrolment.secret}
            </code>
            <Button
              variant='outline'
              size='icon'
              aria-label={t('settings.mfa.enrol.copySecret')}
              onClick={() => copyToClipboard(enrolment.secret, t)}
            >
              <Copy className='h-4 w-4' />
            </Button>
          </div>
        </div>
        <CodeForm
          id='mfa-confirm-code'
          label={t('settings.mfa.enrol.code')}
          submitLabel={t('settings.mfa.enrol.confirm')}
          isPending={mutation.isPending}
          error={mutation.error ? describeMfaError(mutation.error, t) : null}
          onSubmit={(code) => mutation.mutate(code)}
          onCancel={onCancel}
        />
      </CardContent>
    </Card>
  )
}
