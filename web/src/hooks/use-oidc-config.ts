import axiosInstance from '@/api/axiosInstance'
import { useQuery } from '@tanstack/react-query'

interface OidcConfigResponse {
  enabled: boolean
  auto_redirect: boolean
}

async function fetchOidcConfig(): Promise<OidcConfigResponse> {
  const { data } = await axiosInstance.get<OidcConfigResponse>(
    'api/auth/oidc/config'
  )
  return data
}

/**
 * Reads the public OIDC client config from the server.
 * Used by the sign-in page to decide whether to render the "Sign in
 * with SSO" button and whether to auto-redirect to the IdP.
 */
export function useOidcConfig() {
  const { data } = useQuery({
    queryKey: ['oidc-config'],
    queryFn: fetchOidcConfig,
    staleTime: Infinity,
    retry: 1,
  })

  return {
    oidcEnabled: data?.enabled ?? false,
    oidcAutoRedirect: data?.auto_redirect ?? false,
  } as const
}
