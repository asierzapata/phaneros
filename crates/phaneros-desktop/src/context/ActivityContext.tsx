import React, { createContext, useContext, useEffect, useState } from 'react';
import { ActivitySession } from '@/types';
import { mockRecentActivity } from '@/__tests__/mocks/activityMocks';
import { fetchRecentActivity } from '@/lib/backendBridge';

export interface ActivityContextType {
  activity: ActivitySession[];
  isLoading: boolean;
  error: string | null;
}

const ActivityContext = createContext<ActivityContextType | undefined>(undefined);

export interface ActivityProviderProps {
  children: React.ReactNode;
  initialActivity?: ActivitySession[];
}

export const ActivityProvider: React.FC<ActivityProviderProps> = ({
  children,
  initialActivity = mockRecentActivity,
}) => {
  const [activity, setActivity] = useState<ActivitySession[]>(initialActivity);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const loadActivity = () => {
    setIsLoading(true);
    fetchRecentActivity()
      .then((fetched) => {
        if (fetched === null) return;
        setActivity(fetched);
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
    loadActivity();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ActivityContext.Provider value={{ activity, isLoading, error }}>
      {children}
    </ActivityContext.Provider>
  );
};

export const useActivity = (): ActivityContextType => {
  const context = useContext(ActivityContext);
  if (!context) {
    throw new Error('useActivity must be used within an ActivityProvider');
  }
  return context;
};

export { ActivityContext };
