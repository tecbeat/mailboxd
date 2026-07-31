import { z } from 'zod'

const paramEntry = (t: (key: string) => string) =>
  z.object({
    key: z
      .string({ error: t('oauth2.keyIsRequired') })
      .min(1, t('oauth2.keyCannotBeEmpty')),
    value: z
      .string({ error: t('oauth2.valueIsRequired') })
      .min(1, t('oauth2.valueCannotBeEmpty')),
  })

const scopeEntry = (t: (key: string) => string) =>
  z.object({
    value: z
      .string({ error: t('oauth2.valueIsRequired') })
      .min(1, t('oauth2.valueCannotBeEmpty')),
  })

export const getOAuth2Schema = (t: (key: string) => string) =>
  z.object({
    description: z
      .string()
      .max(255, { message: t('oauth2.descriptionMustNotExceed255Characters') })
      .optional(),
    client_id: z
      .string({
        error: t('oauth2.clientIdIsRequired'),
      })
      .min(1, { message: t('oauth2.clientIdCannotBeEmpty') }),
    client_secret: z.string().optional(),
    auth_url: z
      .url({
        error: (issue) =>
          issue.input === undefined || issue.input === ''
            ? t('oauth2.authorizationUrlIsRequired')
            : t('oauth2.invalidAuthorizationUrlFormat'),
      })
      .min(1, { message: t('oauth2.authorizationUrlCannotBeEmpty') }),
    token_url: z
      .url({
        error: (issue) =>
          issue.input === undefined || issue.input === ''
            ? t('oauth2.tokenUrlIsRequired')
            : t('oauth2.invalidTokenUrlFormat'),
      })
      .min(1, { message: t('oauth2.tokenUrlCannotBeEmpty') }),
    redirect_uri: z
      .url({
        error: (issue) =>
          issue.input === undefined || issue.input === ''
            ? t('oauth2.redirectUriIsRequired')
            : t('oauth2.invalidRedirectUriFormat'),
      })
      .min(1, { message: t('oauth2.redirectUriCannotBeEmpty') }),
    scopes: z.array(scopeEntry(t)).optional(),
    extra_params: z.array(paramEntry(t)).optional(),
    enabled: z.boolean(),
    use_proxy: z.number().optional(),
  })

export type OAuth2FormValues = z.infer<ReturnType<typeof getOAuth2Schema>>
