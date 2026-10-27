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
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation } from '@tanstack/react-query'
import { ArrowLeft, KeyRound, LoaderCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { type LoginResult, verify_mfa_login } from '@/api/users/api'
import { toast } from '@/hooks/use-toast'
import { Button } from '@/components/ui/button'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { describeLoginError, describeMfaRestart } from './login-error'
import { getMfaCodeSchema, type MfaCodeFormValues } from './schema'

interface MfaCodeFormProps {
  challenge: string
  /** Called with the final login result once the code was accepted. */
  onVerified: (result: LoginResult) => void
  /** Called when the challenge is gone and the password step must be redone. */
  onRestart: () => void
}

export function MfaCodeForm({
  challenge,
  onVerified,
  onRestart,
}: MfaCodeFormProps) {
  const { t } = useTranslation()
  const form = useForm<MfaCodeFormValues>({
    resolver: zodResolver(getMfaCodeSchema(t)),
    defaultValues: { code: '' },
  })

  const mutation = useMutation({
    mutationFn: (code: string) => verify_mfa_login(challenge, code),
    retry: 0,
  })

  function onSubmit({ code }: MfaCodeFormValues) {
    mutation.mutate(code, {
      onSuccess: (result) => {
        if (result.success) {
          onVerified(result)
        } else if (result.mfa_required) {
          form.setValue('code', '')
          form.setError('code', { message: t('auth.mfa.invalidCode') })
        } else {
          toast({
            variant: 'destructive',
            title: t('auth.loginFailed'),
            description: describeMfaRestart(result, t),
          })
          onRestart()
        }
      },
      onError: (error) => {
        toast({ variant: 'destructive', ...describeLoginError(error, t) })
      },
    })
  }

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(onSubmit)}>
        <div className='grid gap-2'>
          <p className='text-sm text-muted-foreground'>
            {t('auth.mfa.prompt')}
          </p>
          <FormField
            control={form.control}
            name='code'
            render={({ field }) => (
              <FormItem className='space-y-1'>
                <FormLabel>{t('auth.mfa.code')}</FormLabel>
                <FormControl>
                  <Input
                    autoComplete='one-time-code'
                    autoFocus
                    placeholder='123456'
                    {...field}
                  />
                </FormControl>
                <FormMessage />
              </FormItem>
            )}
          />
          <Button className='mt-2' disabled={mutation.isPending}>
            {mutation.isPending ? (
              <LoaderCircle className='animate-spin' />
            ) : (
              <KeyRound className='mr-2' />
            )}
            {t('auth.mfa.verify')}
          </Button>
          <Button
            variant='ghost'
            type='button'
            disabled={mutation.isPending}
            onClick={onRestart}
          >
            <ArrowLeft className='mr-2' />
            {t('auth.mfa.back')}
          </Button>
        </div>
      </form>
    </Form>
  )
}
