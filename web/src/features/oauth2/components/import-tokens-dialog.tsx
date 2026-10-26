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

import { useMemo } from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { useNavigate } from '@tanstack/react-router'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { AxiosError } from 'axios'
import { CircleAlert, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { store_external_oauth2_token } from '@/api/oauth2/api'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Textarea } from '@/components/ui/textarea'
import { ToastAction } from '@/components/ui/toast'
import { VirtualizedSelect } from '@/components/virtualized-select'
import { useCurrentUser } from '@/hooks/use-current-user'
import useMinimalAccountList from '@/hooks/use-minimal-account-list'
import { toast } from '@/hooks/use-toast'
import { OAuth2Entity } from '../data/schema'
import { getImportTokensSchema, type ImportTokensFormValues } from './schema'

interface Props {
  currentRow: OAuth2Entity
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Stores an access and refresh token obtained outside mailboxd (e.g. with
 * email-oauth2-proxy) for an OAuth2 account, using this OAuth2 configuration
 * to refresh it.
 */
export function ImportTokensDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { require_any_permission } = useCurrentUser()
  const { minimalList, isLoading } = useMinimalAccountList()

  // Only OAuth2 accounts can use the tokens, and storing them needs
  // account:manage on that account (checked again by the server).
  const accountsOptions = useMemo(
    () =>
      (minimalList ?? [])
        .filter(
          (account) =>
            account.auth_type === 'OAuth2' &&
            require_any_permission(['system:root', 'account:manage'], account.id),
        )
        .map((account) => ({ label: account.email, value: `${account.id}` })),
    [minimalList, require_any_permission],
  )

  const form = useForm<ImportTokensFormValues>({
    resolver: zodResolver(getImportTokensSchema(t)),
    defaultValues: { access_token: '', refresh_token: '' },
  })

  const importMutation = useMutation({
    mutationFn: (values: ImportTokensFormValues) =>
      store_external_oauth2_token(values.account_id, {
        oauth2_id: currentRow.id,
        access_token: values.access_token,
        refresh_token: values.refresh_token,
      }),
    onSuccess: (_data, values) => {
      toast({
        title: t('oauth2.importTokensSuccess'),
        description: t('oauth2.importTokensSuccessDesc'),
      })
      queryClient.invalidateQueries({ queryKey: ['oauth2-tokens', values.account_id] })
      form.reset()
      onOpenChange(false)
    },
    onError: (error: AxiosError) => {
      const message =
        (error.response?.data as { message?: string })?.message ||
        error.message ||
        t('oauth2.importTokensFailed')
      toast({
        variant: 'destructive',
        title: t('oauth2.importTokensFailed'),
        description: message,
        action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
      })
    },
  })

  const hasAnyAccount = (minimalList?.length ?? 0) > 0
  const hasOAuth2Account = accountsOptions.length > 0

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset()
        onOpenChange(state)
      }}
    >
      <DialogContent className='sm:max-w-2xl'>
        <DialogHeader className='text-left'>
          <DialogTitle>{t('oauth2.importTokensTitle')}</DialogTitle>
          <DialogDescription>{t('oauth2.importTokensDescription')}</DialogDescription>
        </DialogHeader>
        <Form {...form}>
          <form
            id='oauth2-import-tokens-form'
            onSubmit={form.handleSubmit((values) => importMutation.mutate(values))}
            className='flex flex-col space-y-4'
          >
            <Alert className='border-amber-500/40 bg-amber-500/10 text-amber-700 [&>svg]:text-amber-600 dark:text-amber-400'>
              <CircleAlert className='h-4 w-4' />
              <AlertTitle>{t('oauth2.warning')}</AlertTitle>
              <AlertDescription>
                <ul className='list-disc space-y-1 pl-4'>
                  <li>{t('oauth2.importTokensWarningStopExternalTool')}</li>
                  <li>{t('oauth2.importTokensWarningClientMustMatch')}</li>
                  <li>{t('oauth2.importTokensWarningLongLivedCredential')}</li>
                </ul>
              </AlertDescription>
            </Alert>

            <FormField
              control={form.control}
              name='account_id'
              render={({ field }) => (
                <FormItem className='flex flex-col space-y-2'>
                  <FormLabel>{t('oauth2.selectAnAccount')}</FormLabel>
                  {isLoading && (
                    <div className='flex h-16 items-center justify-center'>
                      {t('oauth2.loadingAccounts')}
                    </div>
                  )}
                  {!isLoading && hasOAuth2Account && (
                    <VirtualizedSelect
                      isLoading={isLoading}
                      className='w-full'
                      options={accountsOptions}
                      value={field.value !== undefined ? `${field.value}` : undefined}
                      onSelectOption={(values) =>
                        field.onChange(values.length > 0 ? Number(values[0]) : undefined)
                      }
                      placeholder={t('oauth2.selectAnAccount')}
                    />
                  )}
                  {!isLoading && !hasOAuth2Account && (
                    <div className='flex flex-col items-center justify-center py-4'>
                      <p className='mb-4 text-sm text-muted-foreground'>
                        {hasAnyAccount
                          ? t('oauth2.noOAuth2Accounts')
                          : t('oauth2.noEmailAccountsRegistered')}
                      </p>
                      <Button type='button' onClick={() => navigate({ to: '/accounts' })}>
                        {t('oauth2.createAccount')}
                      </Button>
                    </div>
                  )}
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name='access_token'
              render={({ field }) => (
                <FormItem className='flex flex-col space-y-2'>
                  <FormLabel>{t('oauth2.accessToken')}</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder={t('oauth2.enterTheAccessTokenHere')}
                      rows={3}
                      className='font-mono text-xs'
                      autoComplete='off'
                      spellCheck={false}
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name='refresh_token'
              render={({ field }) => (
                <FormItem className='flex flex-col space-y-2'>
                  <FormLabel>{t('oauth2.refreshToken')}</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder={t('oauth2.enterTheRefreshTokenHere')}
                      rows={3}
                      className='font-mono text-xs'
                      autoComplete='off'
                      spellCheck={false}
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
          </form>
        </Form>
        <DialogFooter>
          <DialogClose asChild>
            <Button type='button' variant='outline' className='h-auto px-2 py-1 text-sm'>
              {t('common.close')}
            </Button>
          </DialogClose>
          {!isLoading && hasOAuth2Account && (
            <Button type='submit' form='oauth2-import-tokens-form' disabled={importMutation.isPending}>
              {importMutation.isPending && <Loader2 className='mr-2 h-4 w-4 animate-spin' />}
              {importMutation.isPending ? t('oauth2.importing') : t('oauth2.actions.importTokens')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
