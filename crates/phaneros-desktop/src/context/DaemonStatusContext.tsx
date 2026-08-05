import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { pingDaemon, startDaemon as startDaemonBridge } from '@/lib/backendBridge';

export type DaemonConnectionState = 'checking' | 'unreachable' | 'reachable';

const POLL_INTERVAL_MS = 3000;
const POST_START_RECHECK_DELAY_MS = 1000;

export interface DaemonStatusContextType {
  connectionState: DaemonConnectionState;
  /** Whether the daemon has at least one drive configured. `null` until the first successful ping. */
  configured: boolean | null;
  lastError: string | null;
  isStartingDaemon: boolean;
  startDaemon: () => Promise<void>;
  refresh: () => Promise<void>;
}

const DaemonStatusContext = createContext<DaemonStatusContextType | undefined>(undefined);

export interface DaemonStatusProviderProps {
  children: React.ReactNode;
  initialConnectionState?: DaemonConnectionState;
  initialConfigured?: boolean | null;
}

export const DaemonStatusProvider: React.FC<DaemonStatusProviderProps> = ({
  children,
  initialConnectionState = 'checking',
  initialConfigured = null,
}) => {
  const [connectionState, setConnectionState] = useState<DaemonConnectionState>(initialConnectionState);
  const [configured, setConfigured] = useState<boolean | null>(initialConfigured);
  const [lastError, setLastError] = useState<string | null>(null);
  const [isStartingDaemon, setIsStartingDaemon] = useState<boolean>(false);
  const pollInFlight = useRef(false);

  const refresh = async () => {
    if (pollInFlight.current) return;
    pollInFlight.current = true;
    try {
      const result = await pingDaemon();
      if (result === null) {
        // Outside Tauri (tests/browser preview): no daemon to reach, behave
        // as if a fully configured daemon is present so the rest of the app
        // renders normally.
        setConnectionState('reachable');
        setConfigured(true);
        setLastError(null);
        return;
      }
      setConnectionState('reachable');
      setConfigured(result.configured);
      setLastError(null);
    } catch (err) {
      setConnectionState('unreachable');
      setConfigured(null);
      setLastError(err instanceof Error ? err.message : String(err));
    } finally {
      pollInFlight.current = false;
    }
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const startDaemon = async () => {
    if (!isTauri()) return;
    setIsStartingDaemon(true);
    try {
      await startDaemonBridge();
      await new Promise((resolve) => setTimeout(resolve, POST_START_RECHECK_DELAY_MS));
      await refresh();
    } catch (err) {
      setLastError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsStartingDaemon(false);
    }
  };

  return (
    <DaemonStatusContext.Provider
      value={{ connectionState, configured, lastError, isStartingDaemon, startDaemon, refresh }}
    >
      {children}
    </DaemonStatusContext.Provider>
  );
};

export const useDaemonStatus = (): DaemonStatusContextType => {
  const context = useContext(DaemonStatusContext);
  if (!context) {
    throw new Error('useDaemonStatus must be used within a DaemonStatusProvider');
  }
  return context;
};

export { DaemonStatusContext };
