import React, { ReactElement } from 'react';
import { render as rtlRender, RenderOptions, RenderResult } from '@testing-library/react';
import { ThemeProvider, ThemeProviderProps } from '@/context/ThemeContext';
import { VaultProvider, VaultProviderProps } from '@/context/VaultContext';
import { TelemetryProvider, TelemetryProviderProps } from '@/context/TelemetryContext';
import { OnboardingProvider, OnboardingProviderProps } from '@/context/OnboardingContext';
import { ViewProvider, ViewProviderProps } from '@/context/ViewContext';

export interface ProviderPropsOptions {
  themeProps?: Partial<ThemeProviderProps>;
  vaultProps?: Partial<VaultProviderProps>;
  telemetryProps?: Partial<TelemetryProviderProps>;
  onboardingProps?: Partial<OnboardingProviderProps>;
  viewProps?: Partial<ViewProviderProps>;
}

export interface CustomRenderOptions extends Omit<RenderOptions, 'wrapper'> {
  providerProps?: ProviderPropsOptions;
}

export const AllProviders: React.FC<{
  children: React.ReactNode;
  providerProps?: ProviderPropsOptions;
}> = ({ children, providerProps }) => {
  return (
    <ThemeProvider {...providerProps?.themeProps}>
      <VaultProvider {...providerProps?.vaultProps}>
        <TelemetryProvider {...providerProps?.telemetryProps}>
          <OnboardingProvider {...providerProps?.onboardingProps}>
            <ViewProvider {...providerProps?.viewProps}>
              {children}
            </ViewProvider>
          </OnboardingProvider>
        </TelemetryProvider>
      </VaultProvider>
    </ThemeProvider>
  );
};

export function customRender(
  ui: ReactElement,
  options?: CustomRenderOptions
): RenderResult {
  const { providerProps, ...renderOptions } = options || {};

  return rtlRender(ui, {
    wrapper: (props) => <AllProviders providerProps={providerProps} {...props} />,
    ...renderOptions,
  });
}

// Re-export everything from @testing-library/react
export * from '@testing-library/react';

// Override default render method
export { customRender as render };
