import React from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { useActivity } from '@/context/ActivityContext';

export const Activity: React.FC = () => {
  const { activity, isLoading, error } = useActivity();

  return (
    <div className="flex flex-col gap-4 p-6 max-w-6xl mx-auto font-sans" data-testid="activity-workspace">
      <h2 className="text-xl font-serif font-bold text-foreground">Recent Sync Activity</h2>
      {error && (
        <p className="text-xs text-red-600 dark:text-red-400" data-testid="activity-error">
          {error}
        </p>
      )}
      <Card elevation="low" className="p-4 bg-card">
        {isLoading && activity.length === 0 ? (
          <p className="text-sm text-muted-foreground py-6 text-center" data-testid="activity-loading">
            Loading activity…
          </p>
        ) : activity.length === 0 ? (
          <p className="text-sm text-muted-foreground py-6 text-center" data-testid="activity-empty">
            No sync activity yet
          </p>
        ) : (
          <div className="divide-y divide-border">
            {activity.map((session) => (
              <div key={session.id} className="py-3 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Badge variant="mono" className="font-mono">{session.driveId}</Badge>
                  <span className="font-semibold text-sm">{session.summary}</span>
                </div>
                <span className="text-xs font-mono text-muted-foreground">{session.timestamp}</span>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
};
