import React, { createContext, useContext, useState } from 'react';
import { OnboardingState, DriveVault } from '@/types';
import { mockOnboardingStep1 } from '@/__tests__/mocks/onboardingMocks';

export interface OnboardingContextType extends OnboardingState {
  setStep: (step: number) => void;
  nextStep: () => void;
  prevStep: () => void;
  completeOnboarding: () => void;
  setDestinationMode: (mode: 'cloud' | 'self-hosted') => void;
  setServerUrl: (url: string) => void;
  setServerToken: (token: string) => void;
  setConnected: (connected: boolean) => void;
  addVault: (vault: Partial<DriveVault>) => void;
  removeVault: (id: string) => void;
}

const OnboardingContext = createContext<OnboardingContextType | undefined>(undefined);

export interface OnboardingProviderProps {
  children: React.ReactNode;
  initialState?: Partial<OnboardingState>;
}

export const OnboardingProvider: React.FC<OnboardingProviderProps> = ({
  children,
  initialState,
}) => {
  const base = { ...mockOnboardingStep1, ...initialState };
  const [currentStep, setCurrentStep] = useState<number>(base.currentStep);
  const [isCompleted, setIsCompleted] = useState<boolean>(base.isCompleted);
  const [destinationMode, setDestinationModeState] = useState<'cloud' | 'self-hosted'>(base.destinationMode);
  const [serverUrl, setServerUrlState] = useState<string>(base.serverUrl);
  const [serverToken, setServerTokenState] = useState<string>(base.serverToken);
  const [isConnected, setIsConnectedState] = useState<boolean>(base.isConnected);
  const [vaults, setVaults] = useState<DriveVault[]>(base.vaults);

  const setStep = (step: number) => {
    if (step >= 1 && step <= 5) {
      setCurrentStep(step);
    }
  };

  const nextStep = () => {
    setCurrentStep((prev) => (prev < 5 ? prev + 1 : prev));
  };

  const prevStep = () => {
    setCurrentStep((prev) => (prev > 1 ? prev - 1 : prev));
  };

  const completeOnboarding = () => {
    setIsCompleted(true);
    setCurrentStep(5);
  };

  const setDestinationMode = (mode: 'cloud' | 'self-hosted') => {
    setDestinationModeState(mode);
  };

  const setServerUrl = (url: string) => {
    setServerUrlState(url);
  };

  const setServerToken = (token: string) => {
    setServerTokenState(token);
  };

  const setConnected = (connected: boolean) => {
    setIsConnectedState(connected);
  };

  const addVault = (vaultPartial: Partial<DriveVault>) => {
    const newVault: DriveVault = {
      id: vaultPartial.id || `vault-${Date.now()}`,
      name: vaultPartial.name || 'New Vault',
      path: vaultPartial.path || '~/Documents/NewVault',
      status: vaultPartial.status || 'synced',
      usedBytes: vaultPartial.usedBytes || 0,
      quotaBytes: undefined,
      fileCount: vaultPartial.fileCount || 0,
    };
    setVaults((prev) => [...prev, newVault]);
  };

  const removeVault = (id: string) => {
    setVaults((prev) => prev.filter((v) => v.id !== id));
  };

  return (
    <OnboardingContext.Provider
      value={{
        currentStep,
        isCompleted,
        destinationMode,
        serverUrl,
        serverToken,
        isConnected,
        vaults,
        setStep,
        nextStep,
        prevStep,
        completeOnboarding,
        setDestinationMode,
        setServerUrl,
        setServerToken,
        setConnected,
        addVault,
        removeVault,
      }}
    >
      {children}
    </OnboardingContext.Provider>
  );
};

export const useOnboarding = (): OnboardingContextType => {
  const context = useContext(OnboardingContext);
  if (!context) {
    throw new Error('useOnboarding must be used within an OnboardingProvider');
  }
  return context;
};

export { OnboardingContext };
