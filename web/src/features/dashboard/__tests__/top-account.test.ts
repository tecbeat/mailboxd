import { describe, it, expect } from 'vitest'
import { resolveTopAccount } from '../top-account'

const accounts = [
  { id: 1, email: 'shared@example.com', name: 'IMAP' },
  { id: 2, email: 'shared@example.com', name: 'Import' },
]

describe('resolveTopAccount', () => {
  it('resolves by account id when two accounts share an email', () => {
    const row = { key: 'shared@example.com', count: 3, account_id: 2 }
    expect(resolveTopAccount(row, accounts)?.name).toBe('Import')
  })

  it('returns null for an unknown account id instead of matching the email', () => {
    const row = { key: 'shared@example.com', count: 3, account_id: 99 }
    expect(resolveTopAccount(row, accounts)).toBeNull()
  })

  it('falls back to the email when the backend sends no account id', () => {
    const row = { key: 'shared@example.com', count: 3 }
    expect(resolveTopAccount(row, accounts)?.id).toBe(1)
  })

  it('returns null while the account list is loading', () => {
    expect(resolveTopAccount({ key: 'x', count: 1, account_id: 1 }, undefined)).toBeNull()
  })
})
