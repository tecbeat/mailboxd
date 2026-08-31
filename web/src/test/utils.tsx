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
