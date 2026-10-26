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
import { LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

interface CodeFormProps {
  id: string
  label: string
  submitLabel: string
  destructive?: boolean
  isPending: boolean
  error: string | null
  onSubmit: (code: string) => void
  onCancel: () => void
}

/** A single code input (TOTP or recovery code) with submit and cancel. */
export function CodeForm({
  id,
  label,
  submitLabel,
  destructive,
  isPending,
  error,
  onSubmit,
  onCancel,
}: CodeFormProps) {
  const { t } = useTranslation()
  const [code, setCode] = useState('')
  const errorId = `${id}-error`

  return (
    <form
      className='space-y-3'
      onSubmit={(e) => {
        e.preventDefault()
        onSubmit(code.trim())
      }}
    >
      <div className='space-y-1'>
        <Label htmlFor={id}>{label}</Label>
        <Input
          id={id}
          autoComplete='one-time-code'
          autoFocus
          placeholder='123456'
          className='w-56'
          value={code}
          onChange={(e) => setCode(e.target.value)}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
        />
        {error && (
          <p id={errorId} className='text-sm text-destructive'>
            {error}
          </p>
        )}
      </div>
      <div className='flex gap-2'>
        <Button
          type='submit'
          variant={destructive ? 'destructive' : 'default'}
          disabled={isPending || !code.trim()}
        >
          {isPending && <LoaderCircle className='mr-2 h-4 w-4 animate-spin' />}
          {submitLabel}
        </Button>
        <Button
          type='button'
          variant='outline'
          disabled={isPending}
          onClick={onCancel}
        >
          {t('common.cancel')}
        </Button>
      </div>
    </form>
  )
}
