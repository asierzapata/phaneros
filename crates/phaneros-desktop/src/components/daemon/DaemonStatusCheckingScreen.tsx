import React from 'react';
import { useTheme } from '@/context/ThemeContext';

export const DaemonStatusCheckingScreen: React.FC = () => {
  const { theme } = useTheme();

  return (
    <div
      className={`min-h-screen bg-background text-foreground bg-dot-grid flex items-center justify-center ${theme}`}
      data-testid="daemon-status-checking-screen"
    >
      <div className="text-sm text-muted-foreground">Checking daemon status…</div>
    </div>
  );
};
