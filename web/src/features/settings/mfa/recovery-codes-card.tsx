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
import { useBlocker } from '@tanstack/react-router'
import { saveAs } from 'file-saver'
import { Copy, Download, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { copyToClipboard } from './clipboard'

interface RecoveryCodesCardProps {
  codes: string[]
  onDone: () => void
}

/** Shows the freshly issued recovery codes; they are never shown again. */
export function RecoveryCodesCard({ codes, onDone }: RecoveryCodesCardProps) {
  const { t } = useTranslation()
  const [saved, setSaved] = useState(false)
  const text = codes.join('\n')

  useBlocker({
    shouldBlockFn: () =>
      !window.confirm(t('settings.mfa.recovery.leaveWarning')),
    enableBeforeUnload: !saved,
    disabled: saved,
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.mfa.recovery.title')}</CardTitle>
        <CardDescription>
          {t('settings.mfa.recovery.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className='space-y-4'>
        <Alert variant='destructive'>
          <TriangleAlert className='h-4 w-4' />
          <AlertDescription>
            {t('settings.mfa.recovery.onlyOnce')}
          </AlertDescription>
        </Alert>
        <ul className='grid max-w-md grid-cols-2 gap-2 font-mono text-sm'>
          {codes.map((code) => (
            <li
              key={code}
              className='rounded border bg-muted px-2 py-1 text-center'
            >
              {code}
            </li>
          ))}
        </ul>
        <div className='flex flex-wrap gap-2'>
          <Button variant='outline' onClick={() => copyToClipboard(text, t)}>
            <Copy className='mr-2 h-4 w-4' />
            {t('settings.mfa.recovery.copy')}
          </Button>
          <Button
            variant='outline'
            onClick={() =>
              saveAs(
                new Blob([`${text}\n`], { type: 'text/plain;charset=utf-8' }),
                'mailboxd-recovery-codes.txt'
              )
            }
          >
            <Download className='mr-2 h-4 w-4' />
            {t('settings.mfa.recovery.download')}
          </Button>
        </div>
        <div className='flex items-center gap-2'>
          <Checkbox
            id='mfa-recovery-saved'
            checked={saved}
            onCheckedChange={(checked) => setSaved(checked === true)}
          />
          <Label htmlFor='mfa-recovery-saved'>
            {t('settings.mfa.recovery.saved')}
          </Label>
        </div>
        <Button disabled={!saved} onClick={onDone}>
          {t('settings.mfa.recovery.done')}
        </Button>
      </CardContent>
    </Card>
  )
}
