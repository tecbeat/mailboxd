import { AxiosError } from 'axios'

export interface LoginErrorMessage {
  title: string
  description: string
}

/** Map a failed login request to the toast title and description to show. */
export function describeLoginError(
  error: unknown,
  t: (key: string) => string
): LoginErrorMessage {
  const status = error instanceof AxiosError ? error.response?.status : undefined
  if (status === 401) {
    return { title: t('auth.loginFailed'), description: t('auth.invalidPassword') }
  }
  if (status === 429) {
    return { title: t('auth.loginFailed'), description: t('auth.tooManyAttempts') }
  }
  return { title: t('auth.somethingWentWrong'), description: (error as Error).message }
}
