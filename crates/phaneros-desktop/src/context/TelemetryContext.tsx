import React, { createContext, useContext, useEffect, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { TelemetryMetrics } from '@/types';
import { mockTelemetry } from '@/__tests__/mocks/telemetryMocks';
import { fetchTelemetry, triggerSync as triggerSyncBridge } from '@/lib/backendBridge';

export interface TelemetryContextType {
  metrics: TelemetryMetrics;
  isSyncing: boolean;
  isLoading: boolean;
  error: string | null;
  triggerSync: () => void;
}

const TelemetryContext = createContext<TelemetryContextType | undefined>(undefined);

export interface TelemetryProviderProps {
  children: React.ReactNode;
  initialMetrics?: TelemetryMetrics;
  initialIsSyncing?: boolean;
}

export const TelemetryProvider: React.FC<TelemetryProviderProps> = ({
  children,
  initialMetrics = mockTelemetry,
  initialIsSyncing = false,
}) => {
  const [metrics, setMetrics] = useState<TelemetryMetrics>(initialMetrics);
  const [isSyncing, setIsSyncing] = useState<boolean>(initialIsSyncing);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const loadMetrics = () => {
    setIsLoading(true);
    fetchTelemetry()
      .then((fetched) => {
        if (fetched === null) return;
        setMetrics(fetched);
        setError(null);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        setIsLoading(false);
      });
  };

  useEffect(() => {
    loadMetrics();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const triggerSync = () => {
    setIsSyncing(true);

    if (!isTauri()) {
      // No backend to talk to (tests/browser preview) — keep the previous
      // simulated-delay behavior so the "Syncing..." state is observable.
      setTimeout(() => setIsSyncing(false), 1500);
      return;
    }

    triggerSyncBridge()
      .then(() => loadMetrics())
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        setIsSyncing(false);
      });
  };

  return (
    <TelemetryContext.Provider value={{ metrics, isSyncing, isLoading, error, triggerSync }}>
      {children}
    </TelemetryContext.Provider>
  );
};

export const useTelemetry = (): TelemetryContextType => {
  const context = useContext(TelemetryContext);
  if (!context) {
    throw new Error('useTelemetry must be used within a TelemetryProvider');
  }
  return context;
};

export { TelemetryContext };
