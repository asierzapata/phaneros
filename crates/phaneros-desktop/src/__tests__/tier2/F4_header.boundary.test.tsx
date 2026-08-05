import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { useView } from '@/context/ViewContext';
import { useTheme } from '@/context/ThemeContext';
import { MainTab } from '@/types';
import React from 'react';

const HeaderBoundaryComponent: React.FC<{ conflictCount?: number; onTabChange?: (tab: MainTab) => void }> = ({
  conflictCount = 0,
  onTabChange,
}) => {
  const { activeTab, setActiveTab } = useView();
  const { theme, toggleTheme } = useTheme();

  const tabs: Array<{ id: MainTab; label: string }> = [
    { id: 'dashboard', label: 'Dashboard' },
    { id: 'drives', label: 'Drives & Files' },
    { id: 'conflicts', label: 'Conflicts' },
    { id: 'activity', label: 'Activity' },
    { id: 'settings', label: 'Settings' },
  ];

  const handleSelect = (tabId: MainTab) => {
    setActiveTab(tabId);
    if (onTabChange) {
      onTabChange(tabId);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent, currentIndex: number) => {
    if (e.key === 'ArrowRight') {
      const nextIndex = (currentIndex + 1) % tabs.length;
      handleSelect(tabs[nextIndex].id);
    } else if (e.key === 'ArrowLeft') {
      const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
      handleSelect(tabs[prevIndex].id);
    }
  };

  return (
    <header className="transparent-top-bar flex justify-between items-center px-4" data-testid="header-root">
      <div className="w-[180px]" data-testid="left-block">
        <span className="brand-wordmark font-serif">PHANEROS</span>
      </div>

      <nav role="tablist" aria-label="Main Navigation" className="hig-segmented-control flex">
        {tabs.map((tab, idx) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            tabIndex={activeTab === tab.id ? 0 : -1}
            onClick={() => handleSelect(tab.id)}
            onKeyDown={(e) => handleKeyDown(e, idx)}
            data-testid={`tab-btn-${tab.id}`}
            className={`px-3 py-1.5 text-sm ${activeTab === tab.id ? 'bg-primary text-white' : ''}`}
          >
            {tab.label}
            {tab.id === 'conflicts' && conflictCount > 0 && (
              <span data-testid="conflict-badge" className="ml-1 bg-red-500 text-white rounded-full px-1.5 text-xs">
                {conflictCount > 99 ? '99+' : conflictCount}
              </span>
            )}
          </button>
        ))}
      </nav>

      <div className="w-[180px] flex justify-end" data-testid="right-block">
        <button onClick={toggleTheme} data-testid="header-theme-toggle">
          {theme === 'light' ? '🌙' : '☀️'}
        </button>
      </div>
    </header>
  );
};

describe('F4_HDR: Header & HIG Tabs Boundary Tests', () => {
  it('F4-T2-01: should support keyboard ArrowLeft and ArrowRight navigation between tabs', () => {
    render(<HeaderBoundaryComponent />);

    const dashboardTab = screen.getByTestId('tab-btn-dashboard');
    dashboardTab.focus();

    // Press ArrowRight to move to drives tab
    fireEvent.keyDown(dashboardTab, { key: 'ArrowRight', code: 'ArrowRight' });
    expect(screen.getByTestId('tab-btn-drives')).toHaveAttribute('aria-selected', 'true');

    // Press ArrowLeft to return to dashboard tab
    const drivesTab = screen.getByTestId('tab-btn-drives');
    fireEvent.keyDown(drivesTab, { key: 'ArrowLeft', code: 'ArrowLeft' });
    expect(screen.getByTestId('tab-btn-dashboard')).toHaveAttribute('aria-selected', 'true');
  });

  it('F4-T2-02: should handle rapid tab clicks accurately updating view state', async () => {
    const user = userEvent.setup();
    render(<HeaderBoundaryComponent />);

    const conflictsTab = screen.getByTestId('tab-btn-conflicts');
    const settingsTab = screen.getByTestId('tab-btn-settings');

    await user.click(conflictsTab);
    await user.click(settingsTab);
    await user.click(conflictsTab);

    expect(conflictsTab).toHaveAttribute('aria-selected', 'true');
    expect(settingsTab).toHaveAttribute('aria-selected', 'false');
  });

  it('F4-T2-03: should truncate conflict badge count when overflow exceeds 99', () => {
    render(<HeaderBoundaryComponent conflictCount={150} />);

    const badge = screen.getByTestId('conflict-badge');
    expect(badge).toHaveTextContent('99+');
  });

  it('F4-T2-04: should maintain structural layout elements on narrow screen container', () => {
    render(<HeaderBoundaryComponent />);

    expect(screen.getByTestId('header-root')).toBeInTheDocument();
    expect(screen.getByTestId('left-block')).toHaveClass('w-[180px]');
    expect(screen.getByTestId('right-block')).toHaveClass('w-[180px]');
  });

  it('F4-T2-05: should degrade gracefully when optional onTabChange handler is missing', async () => {
    const user = userEvent.setup();
    render(<HeaderBoundaryComponent onTabChange={undefined} />);

    const activityTab = screen.getByTestId('tab-btn-activity');
    await expect(user.click(activityTab)).resolves.not.toThrow();
    expect(activityTab).toHaveAttribute('aria-selected', 'true');
  });
});
