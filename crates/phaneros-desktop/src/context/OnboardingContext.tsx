import React, { createContext, useContext, useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { OnboardingState, DriveVault } from '@/types';
import { mockOnboardingStep1 } from '@/__tests__/mocks/onboardingMocks';
import { addVaultRemote, loadOnboardingState, pingDaemon, saveOnboardingState } from '@/lib/backendBridge';

export interface OnboardingContextType extends OnboardingState {
  setStep: (step: number) => void;
  nextStep: () => void;
  prevStep: () => void;
  completeOnboarding: () => Promise<void>;
  setDestinationMode: (mode: 'cloud' | 'self-hosted') => void;
  setServerUrl: (url: string) => void;
  setServerToken: (token: string) => void;
  setConnected: (connected: boolean) => void;
  testConnection: () => Promise<void>;
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
  const [isTestingConnection, setIsTestingConnection] = useState<boolean>(base.isTestingConnection);
  const [connectionError, setConnectionError] = useState<string | null>(base.connectionError);
  const [isCompleting, setIsCompleting] = useState<boolean>(base.isCompleting);
  const [completionError, setCompletionError] = useState<string | null>(base.completionError);
  const [vaults, setVaults] = useState<DriveVault[]>(base.vaults);

  // Hydrate persisted onboarding completion so the wizard doesn't re-run on
  // every launch. No-ops (and keeps in-memory defaults) outside Tauri.
  useEffect(() => {
    if (!isTauri()) return;
    loadOnboardingState()
      .then((saved) => {
        if (!saved) return;
        setIsCompleted(saved.isCompleted);
        setDestinationModeState(saved.destinationMode);
        setServerUrlState(saved.serverUrl);
      })
      .catch(() => {
        // Best-effort hydration: if the daemon bridge isn't available yet
        // (e.g. a test-only isTauri() mock with no invoke() backing it),
        // just keep the in-memory defaults.
      });
  }, []);

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

  const testConnection = async () => {
    setConnectionError(null);

    if (!isTauri()) {
      // No daemon to reach outside a real app shell (tests/browser preview):
      // fall back to the previous field-presence check.
      if (destinationMode === 'self-hosted' && (!serverUrl || !serverToken)) {
        setIsConnectedState(false);
        return;
      }
      setIsConnectedState(true);
      return;
    }

    setIsTestingConnection(true);
    try {
      await pingDaemon();
      setIsConnectedState(true);
    } catch (err) {
      setIsConnectedState(false);
      setConnectionError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsTestingConnection(false);
    }
  };

  const completeOnboarding = async () => {
    setCompletionError(null);
    setIsCompleting(true);

    // Best-effort: provision real vaults and persist completion, but a
    // daemon-down/unreachable error shouldn't trap the user on this screen —
    // surface it and still let them into the app.
    if (isTauri()) {
      const failures: string[] = [];
      for (const vault of vaults) {
        try {
          await addVaultRemote(
            vault.id,
            vault.path,
            destinationMode === 'self-hosted' ? serverUrl : undefined,
            destinationMode === 'self-hosted' ? serverToken : undefined
          );
        } catch (err) {
          failures.push(`${vault.name}: ${err instanceof Error ? err.message : String(err)}`);
        }
      }
      if (failures.length > 0) {
        setCompletionError(failures.join('; '));
      }
      try {
        await saveOnboardingState({ isCompleted: true, destinationMode, serverUrl });
      } catch {
        // Non-fatal: onboarding will just re-run on next launch.
      }
    }

    setIsCompleted(true);
    setCurrentStep(5);
    setIsCompleting(false);
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
        isTestingConnection,
        connectionError,
        isCompleting,
        completionError,
        vaults,
        setStep,
        nextStep,
        prevStep,
        completeOnboarding,
        setDestinationMode,
        setServerUrl,
        setServerToken,
        setConnected,
        testConnection,
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
