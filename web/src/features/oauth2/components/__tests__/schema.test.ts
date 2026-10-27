import { describe, it, expect } from 'vitest'
import { getImportTokensSchema, getOAuth2Schema } from '../schema'

const t = (key: string) => key

describe('OAuth2 Form Schema', () => {
  const schema = getOAuth2Schema(t)

  const validData = {
    client_id: 'my-client-id',
    auth_url: 'https://accounts.example.com/o/oauth2/auth',
    token_url: 'https://oauth2.example.com/token',
    redirect_uri: 'https://myapp.example.com/oauth2/callback',
    enabled: true,
  }

  describe('client_id field', () => {
    it('rejects empty client_id', () => {
      const result = schema.safeParse({ ...validData, client_id: '' })
      expect(result.success).toBe(false)
    })

    it('accepts valid client_id', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })
  })

  describe('client_secret field', () => {
    it('accepts undefined client_secret', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts provided client_secret', () => {
      const result = schema.safeParse({
        ...validData,
        client_secret: 'my-secret',
      })
      expect(result.success).toBe(true)
    })
  })

  describe('auth_url field', () => {
    it('rejects empty auth_url', () => {
      const result = schema.safeParse({ ...validData, auth_url: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for auth_url', () => {
      const result = schema.safeParse({
        ...validData,
        auth_url: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })

    it('accepts valid auth_url', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })
  })

  describe('token_url field', () => {
    it('rejects empty token_url', () => {
      const result = schema.safeParse({ ...validData, token_url: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for token_url', () => {
      const result = schema.safeParse({
        ...validData,
        token_url: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })
  })

  describe('redirect_uri field', () => {
    it('rejects empty redirect_uri', () => {
      const result = schema.safeParse({ ...validData, redirect_uri: '' })
      expect(result.success).toBe(false)
    })

    it('rejects invalid URL format for redirect_uri', () => {
      const result = schema.safeParse({
        ...validData,
        redirect_uri: 'not-a-url',
      })
      expect(result.success).toBe(false)
    })
  })

  describe('scopes field', () => {
    it('accepts empty scopes array', () => {
      const result = schema.safeParse({ ...validData, scopes: [] })
      expect(result.success).toBe(true)
    })

    it('accepts valid scopes', () => {
      const result = schema.safeParse({
        ...validData,
        scopes: [{ value: 'https://mail.google.com/' }],
      })
      expect(result.success).toBe(true)
    })

    it('rejects scope with empty value', () => {
      const result = schema.safeParse({
        ...validData,
        scopes: [{ value: '' }],
      })
      expect(result.success).toBe(false)
    })
  })

  describe('extra_params field', () => {
    it('accepts empty extra_params array', () => {
      const result = schema.safeParse({ ...validData, extra_params: [] })
      expect(result.success).toBe(true)
    })

    it('accepts valid extra_params', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: 'access_type', value: 'offline' }],
      })
      expect(result.success).toBe(true)
    })

    it('rejects param with empty key', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: '', value: 'offline' }],
      })
      expect(result.success).toBe(false)
    })

    it('rejects param with empty value', () => {
      const result = schema.safeParse({
        ...validData,
        extra_params: [{ key: 'access_type', value: '' }],
      })
      expect(result.success).toBe(false)
    })
  })

  describe('enabled field', () => {
    it('accepts enabled: true', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts enabled: false', () => {
      const result = schema.safeParse({ ...validData, enabled: false })
      expect(result.success).toBe(true)
    })
  })

  describe('description field', () => {
    it('rejects description longer than 255 characters', () => {
      const result = schema.safeParse({
        ...validData,
        description: 'a'.repeat(256),
      })
      expect(result.success).toBe(false)
    })

    it('accepts description of exactly 255 characters', () => {
      const result = schema.safeParse({
        ...validData,
        description: 'a'.repeat(255),
      })
      expect(result.success).toBe(true)
    })
  })

  describe('use_proxy field', () => {
    it('accepts undefined use_proxy', () => {
      const result = schema.safeParse(validData)
      expect(result.success).toBe(true)
    })

    it('accepts numeric use_proxy', () => {
      const result = schema.safeParse({ ...validData, use_proxy: 1 })
      expect(result.success).toBe(true)
    })
  })
})

describe('Import Tokens Form Schema', () => {
  const schema = getImportTokensSchema(t)

  const validData = {
    account_id: 42,
    access_token: 'ya29.a0AfH6SMBexample',
    refresh_token: '1//0exampleRefreshToken',
  }

  const firstMessage = (result: ReturnType<typeof schema.safeParse>) =>
    result.success ? undefined : result.error.issues[0].message

  it('accepts a complete payload', () => {
    expect(schema.safeParse(validData).success).toBe(true)
  })

  it.each([
    ['missing', { access_token: 'a', refresh_token: 'r' }],
    ['non-numeric', { ...validData, account_id: '42' }],
  ])('rejects a %s account_id', (_, data) => {
    const result = schema.safeParse(data)
    expect(result.success).toBe(false)
    expect(firstMessage(result)).toBe('oauth2.accountIsRequired')
  })

  it.each([
    ['access_token', 'oauth2.accessTokenCannotBeEmpty'],
    ['refresh_token', 'oauth2.refreshTokenCannotBeEmpty'],
  ] as const)('rejects an empty or blank %s', (field, message) => {
    for (const value of ['', '  \n\t ']) {
      const result = schema.safeParse({ ...validData, [field]: value })
      expect(result.success).toBe(false)
      expect(firstMessage(result)).toBe(message)
    }
  })

  it('trims surrounding whitespace off the tokens', () => {
    const result = schema.safeParse({
      ...validData,
      access_token: '  ya29.token  ',
      refresh_token: '\n1//0token\n',
    })
    expect(result.success && result.data).toMatchObject({
      access_token: 'ya29.token',
      refresh_token: '1//0token',
    })
  })
})
