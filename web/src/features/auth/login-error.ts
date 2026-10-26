import { AxiosError } from 'axios'
import type { LoginResult } from '@/api/users/api'

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

// Why the second step ended, by the server's `error_code`.
const MFA_RESTART_CODES: Record<string, string> = {
  mfa_too_many_attempts: 'auth.mfa.tooManyCodes',
  mfa_challenge_invalid: 'auth.mfa.challengeExpired',
}

/** Describe why the second step ended and the user has to sign in again. */
export function describeMfaRestart(
  result: Pick<LoginResult, 'error_code' | 'error_message'>,
  t: (key: string) => string
): string {
  const key = result.error_code ? MFA_RESTART_CODES[result.error_code] : undefined
  return key ? t(key) : (result.error_message ?? t('auth.mfa.challengeExpired'))
}
