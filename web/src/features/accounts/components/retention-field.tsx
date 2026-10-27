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
import { useFormContext, useWatch } from 'react-hook-form'
import { AlertTriangle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Alert, AlertDescription } from '@/components/ui/alert'
import {
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { MAX_RETENTION_DAYS, parseRetentionDays } from './schema'

export type RetentionFormValues = { retention_days?: number }

/** The account's retention period, for any form with a `retention_days` field. */
export function RetentionField() {
  const { t } = useTranslation()
  const { control } = useFormContext<RetentionFormValues>()
  const days = useWatch({ control, name: 'retention_days' })

  return (
    <FormField
      control={control}
      name='retention_days'
      render={({ field }) => (
        <FormItem>
          <FormLabel>{t('accounts.retentionDays')}</FormLabel>
          <FormControl>
            <Input
              type='number'
              min={1}
              max={MAX_RETENTION_DAYS}
              step={1}
              placeholder={t('accounts.retentionOff')}
              name={field.name}
              ref={field.ref}
              onBlur={field.onBlur}
              value={field.value ?? ''}
              onChange={(e) =>
                field.onChange(parseRetentionDays(e.target.value))
              }
            />
          </FormControl>
          <FormDescription>
            {t('accounts.retentionDaysDescription')}
          </FormDescription>
          <FormMessage />
          {days !== undefined && (
            <Alert variant='destructive'>
              <AlertTriangle className='h-4 w-4' />
              <AlertDescription>
                {t('accounts.retentionWarning')}
              </AlertDescription>
            </Alert>
          )}
        </FormItem>
      )}
    />
  )
}
