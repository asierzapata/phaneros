import React from 'react';
import { Card } from '@/components/ui/Card';
import { useTheme } from '@/context/ThemeContext';

export const Settings: React.FC = () => {
  const { theme, toggleTheme } = useTheme();

  return (
    <div className="flex flex-col gap-4 p-6 max-w-6xl mx-auto font-sans" data-testid="settings-workspace">
      <h2 className="text-xl font-serif font-bold text-foreground">Application Settings</h2>
      <Card elevation="low" className="p-4 flex flex-col gap-4 bg-card">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-semibold text-sm">Appearance Mode</h3>
            <p className="text-xs text-muted-foreground">Switch between light and dark background substrates.</p>
          </div>
          <button
            type="button"
            onClick={toggleTheme}
            className="px-3 py-1.5 text-xs font-semibold rounded border border-border bg-muted hover:bg-accent"
          >
            Current: {theme.toUpperCase()}
          </button>
        </div>
      </Card>
    </div>
  );
};
