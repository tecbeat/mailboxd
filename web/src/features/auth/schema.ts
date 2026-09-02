import { z } from 'zod'
import type { TOptions } from 'i18next'

export const getFormSchema = (
  t: (key: string, options?: TOptions) => string
) =>
  z.object({
    username: z
      .string()
      .min(1, { message: t('validation.pleaseEnterUsernameOrEmail') }),
    password: z
      .string()
      .min(1, { message: t('validation.pleaseEnterPassword') })
      .min(4, { message: t('validation.passwordMinLength', { min: 4 }) }),
  })

export type LoginFormValues = z.infer<ReturnType<typeof getFormSchema>>
