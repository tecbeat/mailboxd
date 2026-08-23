//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//

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
