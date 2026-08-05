import React from 'react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { mockTrayRecentActivity } from '@/__tests__/mocks/trayMocks';

export const Activity: React.FC = () => {
  return (
    <div className="flex flex-col gap-4 p-6 max-w-6xl mx-auto font-sans" data-testid="activity-workspace">
      <h2 className="text-xl font-serif font-bold text-foreground">File Activity Log</h2>
      <Card elevation="low" className="p-4 bg-card">
        <div className="divide-y divide-border">
          {mockTrayRecentActivity.map((item) => (
            <div key={item.id} className="py-3 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <Badge variant="mono" className="font-mono">{item.ext}</Badge>
                <span className="font-semibold text-sm">{item.name}</span>
                <span className="text-xs text-muted-foreground">({item.action})</span>
              </div>
              <span className="text-xs font-mono text-muted-foreground">{item.timestamp}</span>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
};
