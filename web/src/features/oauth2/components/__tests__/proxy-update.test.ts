import { describe, it, expect } from 'vitest'
import { proxyUpdateFields } from '../proxy-update'

describe('proxyUpdateFields', () => {
  it('sends the selected proxy', () => {
    expect(proxyUpdateFields(4)).toEqual({ use_proxy: 4 })
  })

  it('clears the proxy when "No proxy" is selected', () => {
    // An absent use_proxy keeps the stored proxy, so clearing must be explicit.
    expect(proxyUpdateFields(undefined)).toEqual({ clear_proxy: true })
  })
})
