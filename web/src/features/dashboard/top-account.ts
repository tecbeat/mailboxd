import { MinimalAccount } from '@/api/account/api'
import { Group } from '@/api/system/api'

/**
 * Resolve the account a top_accounts row refers to. The row key is the
 * account email, which is not unique across accounts (e.g. a NoSync import
 * and an IMAP account for the same mailbox), so look it up by account_id when
 * the backend provides one and fall back to the email only when it doesn't.
 */
export function resolveTopAccount(
  row: Group,
  accounts: MinimalAccount[] | undefined
): MinimalAccount | null {
  if (!accounts) return null
  if (row.account_id != null) {
    return accounts.find((a) => a.id === row.account_id) ?? null
  }
  return accounts.find((a) => a.email === row.key) ?? null
}
