import React, { useEffect } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useOnboarding } from '@/context/OnboardingContext';
import { useView } from '@/context/ViewContext';
import { Header } from '@/components/main/Header';
import { Dashboard } from '@/components/main/Dashboard';
import { DrivesFiles } from '@/components/main/DrivesFiles';
import { Conflicts } from '@/components/main/Conflicts';
import { Activity } from '@/components/main/Activity';
import { Settings } from '@/components/main/Settings';
import { TrayPopup } from '@/components/tray/TrayPopup';
import { OnboardingWizard } from '@/components/onboarding/OnboardingWizard';
import { useTheme } from '@/context/ThemeContext';
import { NAVIGATE_TO_TAB_EVENT } from '@/lib/trayBridge';
import { MainTab } from '@/types';

const isTrayWindow = (): boolean => {
  try {
    return getCurrentWindow().label === 'tray';
  } catch {
    // Not running inside a Tauri webview (e.g. tests, browser dev preview)
    return false;
  }
};

export const AppContent: React.FC = () => {
  const { isCompleted } = useOnboarding();
  const { activeTab, setActiveTab } = useView();
  const { theme } = useTheme();
  const isTray = isTrayWindow();

  useEffect(() => {
    if (isTray || !isTauri()) return;
    const unlisten = listen<{ tab: MainTab }>(NAVIGATE_TO_TAB_EVENT, (event) => {
      setActiveTab(event.payload.tab);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isTray, setActiveTab]);

  if (isTray) {
    return <TrayPopup />;
  }

  // If onboarding is not completed, render OnboardingWizard
  if (!isCompleted) {
    return (
      <div className={`min-h-screen bg-background text-foreground bg-dot-grid flex items-center justify-center p-6 ${theme}`}>
        <OnboardingWizard />
      </div>
    );
  }

  // Render view corresponding to active tab
  const renderMainView = () => {
    switch (activeTab) {
      case 'dashboard':
        return <Dashboard />;
      case 'drives':
        return <DrivesFiles />;
      case 'conflicts':
        return <Conflicts />;
      case 'activity':
        return <Activity />;
      case 'settings':
        return <Settings />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <div className={`min-h-screen bg-background text-foreground bg-dot-grid flex flex-col ${theme}`} data-testid="main-app-container">
      <Header />
      <main className="flex-1 p-4">{renderMainView()}</main>
    </div>
  );
};

export default AppContent;
