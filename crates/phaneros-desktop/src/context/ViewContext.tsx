import React, { createContext, useContext, useState } from 'react';
import { MainTab, ViewState } from '@/types';

export interface ViewContextType extends ViewState {}

const ViewContext = createContext<ViewContextType | undefined>(undefined);

export interface ViewProviderProps {
  children: React.ReactNode;
  initialTab?: MainTab;
}

export const ViewProvider: React.FC<ViewProviderProps> = ({
  children,
  initialTab = 'dashboard',
}) => {
  const [activeTab, setActiveTab] = useState<MainTab>(initialTab);

  return (
    <ViewContext.Provider value={{ activeTab, setActiveTab }}>
      {children}
    </ViewContext.Provider>
  );
};

export const useView = (): ViewContextType => {
  const context = useContext(ViewContext);
  if (!context) {
    throw new Error('useView must be used within a ViewProvider');
  }
  return context;
};

export { ViewContext };
