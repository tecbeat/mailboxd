import { z } from 'zod'

const encryptionSchema = z.union([
  z.literal('Ssl'),
  z.literal('StartTls'),
  z.literal('None'),
])

const authTypeSchema = z.union([
  z.literal('Password'),
  z.literal('OAuth2'),
])

export const getAuthConfigSchema = (isEdit: boolean, t: (key: string) => string) =>
  z
    .object({
      auth_type: authTypeSchema,
      password: z.string().optional(),
    })
    .refine(
      (data) => {
        if (data.auth_type === 'Password' && !isEdit) {
          return !!data.password?.trim()
        }
        return true
      },
      {
        message: t('validation.passwordRequired'),
        path: ['password'],
      }
    )

export const getImapConfigSchema = (isEdit: boolean, t: (key: string) => string) =>
  z.object({
    host: z
      .string({ error: t('validation.imapHostRequired') })
      .min(1, { message: t('validation.imapHostCannotBeEmpty') }),
    port: z
      .number()
      .int()
      .min(0, { message: t('validation.imapPortMustBePositive') })
      .max(65535, { message: t('validation.imapPortMustBeLessThan65536') }),
    encryption: encryptionSchema,
    auth: getAuthConfigSchema(isEdit, t),
    use_proxy: z.number().optional(),
  })

const relativeDateSchema = (t: (key: string) => string) =>
  z.object({
    unit: z.enum(['Days', 'Months', 'Years'], {
      message: t('accounts.selectUnit'),
    }),
    value: z
      .number({ error: t('accounts.enterValue') })
      .int()
      .min(1, t('accounts.mustBeAtLeast1')),
  })

const dateSelectionSchema = (t: (key: string) => string) =>
  z
    .object({
      // nullish: the API returns the unused half as null
      fixed: z
        .string({ error: t('accounts.selectDate') })
        .min(1, { message: t('accounts.selectDate') })
        .nullish(),
      relative: relativeDateSchema(t).nullish(),
    })
    .optional()

const filterRuleSchema = z.object({
  include: z.array(z.string()),
  exclude: z.array(z.string()),
})

const archiveRulesSchema = z.object({
  enabled: z.boolean(),
  senders: filterRuleSchema,
  subjects: filterRuleSchema,
  skip_larger_than: z.number().int().positive().optional(),
  spam_headers: z.array(z.string()),
})

/** The longest retention period the server accepts. */
export const MAX_RETENTION_DAYS = 36500

/** Days to keep archived mail; unset keeps everything. */
export const getRetentionDaysSchema = (t: (key: string) => string) => {
  const message = t('validation.retentionDaysRange')
  return z
    .number({ error: message })
    .int({ message })
    .min(1, { message })
    .max(MAX_RETENTION_DAYS, { message })
    .optional()
}

/** The days typed into the retention field; an empty field turns retention off. */
export function parseRetentionDays(value: string): number | undefined {
  return value.trim() === '' ? undefined : Number(value)
}

export const getAccountSchema = (isEdit: boolean, t: (key: string) => string) =>
  z.object({
    account_name: z.string().optional(),
    login_name: z.string().optional(),
    email: z
      .email({
        error: (issue) =>
          issue.input === undefined || issue.input === ''
            ? t('validation.emailRequired')
            : t('validation.invalidEmail'),
      }),
    imap: getImapConfigSchema(isEdit, t),
    enabled: z.boolean(),
    use_dangerous: z.boolean(),
    date_since: dateSelectionSchema(t).optional(),
    date_before: relativeDateSchema(t).optional(),
    download_interval_min: z
      .number({
        error: t('validation.incrementalSyncMustBeNumber'),
      })
      .int()
      .min(10, {
        message: t('validation.incrementalSyncMustBeAtLeast10'),
      }),
    download_batch_size: z
      .number({
        error: t('validation.singleRequestBatchSizeMustBeNumber'),
      })
      .int()
      .min(10, {
        message: t('validation.singleRequestBatchSizeTooSmall'),
      })
      .max(200, {
        message: t('validation.singleRequestBatchSizeTooLarge'),
      }),
    max_email_size_bytes: z
      .number({
        error: t('validation.maxEmailSizeMustBeNumber'),
      })
      .int()
      .min(1 * 1024 * 1024, { message: t('validation.maxEmailSizeTooSmall') })
      .max(100 * 1024 * 1024, { message: t('validation.maxEmailSizeTooLarge') }),
    auto_download_new_mailboxes: z.boolean(),
    download_schedule: z
      .string()
      .optional()
      .refine(
        (val) => {
          if (!val || val.trim() === '') return true;
          const fields = val.trim().split(/\s+/);
          if (fields.length < 6) return false;
          return true;
        },
        { message: t('validation.invalidCronExpression') }
      ),
    archive_rules: archiveRulesSchema.optional(),
    retention_days: getRetentionDaysSchema(t),
  })

export type AccountFormValues = z.infer<
  ReturnType<typeof getAccountSchema>
>
