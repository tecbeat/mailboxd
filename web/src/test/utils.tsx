//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, type RenderOptions } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { I18nextProvider } from 'react-i18next';

import { SidebarProvider } from '@/components/ui/sidebar';
import i18n from '@/i18n';

function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

interface RenderWithProvidersOptions extends Omit<RenderOptions, 'wrapper'> {
  /** Inject a pre-seeded QueryClient so a test can assert cache state after mutations. */
  queryClient?: QueryClient;
}

/**
 * Render a component with the app's query and i18n providers. Each test gets a
 * fresh QueryClient unless one is injected; the client is returned so tests can
 * seed queries and assert invalidation.
 */
export function renderWithProviders(
  ui: ReactElement,
  { queryClient, ...options }: RenderWithProvidersOptions = {},
) {
  const client = queryClient ?? createQueryClient();
  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={client}>
        <I18nextProvider i18n={i18n}>
          <SidebarProvider>{children}</SidebarProvider>
        </I18nextProvider>
      </QueryClientProvider>
    );
  }
  return { queryClient: client, ...render(ui, { wrapper: Wrapper, ...options }) };
}
