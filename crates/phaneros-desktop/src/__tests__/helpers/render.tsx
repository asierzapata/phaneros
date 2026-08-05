import React, { ReactElement } from 'react';
import { render as rtlRender, RenderOptions, RenderResult } from '@testing-library/react';
import { ThemeProvider, ThemeProviderProps } from '@/context/ThemeContext';
import { VaultProvider, VaultProviderProps } from '@/context/VaultContext';
import { TelemetryProvider, TelemetryProviderProps } from '@/context/TelemetryContext';
import { OnboardingProvider, OnboardingProviderProps } from '@/context/OnboardingContext';
import { ViewProvider, ViewProviderProps } from '@/context/ViewContext';
import { ActivityProvider, ActivityProviderProps } from '@/context/ActivityContext';
import { DaemonStatusProvider, DaemonStatusProviderProps } from '@/context/DaemonStatusContext';

export interface ProviderPropsOptions {
  themeProps?: Partial<ThemeProviderProps>;
  vaultProps?: Partial<VaultProviderProps>;
  telemetryProps?: Partial<TelemetryProviderProps>;
  onboardingProps?: Partial<OnboardingProviderProps>;
  viewProps?: Partial<ViewProviderProps>;
  activityProps?: Partial<ActivityProviderProps>;
  daemonStatusProps?: Partial<DaemonStatusProviderProps>;
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
      <DaemonStatusProvider
        initialConnectionState="reachable"
        initialConfigured={true}
        {...providerProps?.daemonStatusProps}
      >
        <VaultProvider {...providerProps?.vaultProps}>
          <TelemetryProvider {...providerProps?.telemetryProps}>
            <ActivityProvider {...providerProps?.activityProps}>
              <OnboardingProvider {...providerProps?.onboardingProps}>
                <ViewProvider {...providerProps?.viewProps}>
                  {children}
                </ViewProvider>
              </OnboardingProvider>
            </ActivityProvider>
          </TelemetryProvider>
        </VaultProvider>
      </DaemonStatusProvider>
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
