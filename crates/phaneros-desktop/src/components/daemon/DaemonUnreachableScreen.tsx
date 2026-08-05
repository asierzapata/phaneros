import React from 'react';
import { useTheme } from '@/context/ThemeContext';
import { useDaemonStatus } from '@/context/DaemonStatusContext';

export const DaemonUnreachableScreen: React.FC = () => {
  const { theme } = useTheme();
  const { lastError, startDaemon, isStartingDaemon } = useDaemonStatus();

  return (
    <div
      className={`min-h-screen bg-background text-foreground bg-dot-grid flex items-center justify-center ${theme}`}
      data-testid="daemon-unreachable-screen"
    >
      <div className="max-w-md w-full mx-auto px-6 py-8 bg-card border border-border rounded-xl shadow-card flex flex-col items-center gap-4 text-center">
        <div className="font-serif text-2xl font-bold text-foreground">
          Phaneros daemon isn&rsquo;t running
        </div>
        <div className="text-sm text-muted-foreground">
          The desktop app needs the background sync daemon (<code>phanerosd</code>) to be
          running. Start it to continue.
        </div>
        {lastError && (
          <div className="text-xs text-muted-foreground bg-secondary rounded-lg px-3 py-2 w-full" data-testid="daemon-unreachable-error">
            {lastError}
          </div>
        )}
        <button
          type="button"
          onClick={() => startDaemon()}
          disabled={isStartingDaemon}
          className="w-full h-10 rounded-lg bg-primary text-primary-foreground text-sm font-semibold shadow-card hover:-translate-y-px transition-all disabled:opacity-60 disabled:cursor-not-allowed"
          data-testid="daemon-unreachable-start-button"
        >
          {isStartingDaemon ? 'Starting…' : 'Start Daemon'}
        </button>
      </div>
    </div>
  );
};
