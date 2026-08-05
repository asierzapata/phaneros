import React from 'react';
import { useView } from '@/context/ViewContext';
import { useTheme } from '@/context/ThemeContext';
import { SegmentedControl } from '@/components/ui/SegmentedControl';
import { MainTab } from '@/types';

export const Header: React.FC = () => {
  const { activeTab, setActiveTab } = useView();
  const { theme, toggleTheme } = useTheme();

  const tabOptions: Array<{ value: MainTab; label: string }> = [
    { value: 'dashboard', label: 'Dashboard' },
    { value: 'drives', label: 'Drives & Files' },
    { value: 'conflicts', label: 'Conflicts' },
    { value: 'activity', label: 'Activity' },
    { value: 'settings', label: 'Settings' },
  ];

  return (
    <header
      className="w-full h-14 flex items-center justify-between px-6 bg-transparent border-b border-border font-sans"
      data-testid="main-header"
    >
      {/* Left fixed 180px side block for title & brand */}
      <div className="w-[180px] flex items-center gap-2" data-testid="header-left-block">
        <h1 className="text-lg font-serif font-bold tracking-widest text-foreground">
          PHANEROS
        </h1>
      </div>

      {/* Center 5 Apple HIG segmented tabs */}
      <div className="flex-1 flex justify-center" data-testid="header-center-tabs">
        <SegmentedControl
          options={tabOptions}
          value={activeTab}
          onChange={(val) => setActiveTab(val as MainTab)}
        />
      </div>

      {/* Right fixed 180px side block for theme toggle */}
      <div className="w-[180px] flex items-center justify-end" data-testid="header-right-block">
        <button
          type="button"
          onClick={toggleTheme}
          aria-label={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
          className="p-2 rounded-lg border border-border bg-card hover:bg-accent text-foreground transition-colors text-xs font-medium"
        >
          {theme === 'light' ? '🌙 Dark' : '☀️ Light'}
        </button>
      </div>
    </header>
  );
};
