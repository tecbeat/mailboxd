import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, type RenderOptions } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { I18nextProvider } from 'react-i18next';

import { SidebarProvider } from '@/components/ui/sidebar';
import i18n from '@/i18n';

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <I18nextProvider i18n={i18n}>
          <SidebarProvider>{children}</SidebarProvider>
        </I18nextProvider>
      </QueryClientProvider>
    );
  };
}

/** Render a component with the app's query and i18n providers, each test getting a fresh QueryClient. */
export function renderWithProviders(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'>,
) {
  return render(ui, { wrapper: createWrapper(), ...options });
}
