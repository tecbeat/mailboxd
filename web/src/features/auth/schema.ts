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

// Six TOTP digits or an eight-symbol recovery code; spaces and dashes are
// ignored the same way the server ignores them.
function isWellFormedCode(value: string): boolean {
  const compact = value.replace(/[\s-]/g, '')
  return /^\d{6}$/.test(compact) || /^[A-Za-z0-9]{8}$/.test(compact)
}

export const getMfaCodeSchema = (t: (key: string) => string) =>
  z.object({
    code: z
      .string()
      .refine(isWellFormedCode, { message: t('auth.mfa.codeFormat') }),
  })

export type MfaCodeFormValues = z.infer<ReturnType<typeof getMfaCodeSchema>>
