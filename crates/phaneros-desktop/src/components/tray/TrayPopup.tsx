import React, { useEffect, useRef } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { useVault } from '@/context/VaultContext';
import { useTelemetry } from '@/context/TelemetryContext';
import { mockTrayRecentActivity } from '@/__tests__/mocks/trayMocks';
import { navigateMainWindowToTab } from '@/lib/trayBridge';

const TRAY_WIDTH = 380;

/**
 * The tray window is transparent (see tauri.conf.json), so only the rounded
 * card itself should paint a background — otherwise the opaque,
 * square-cornered document background shows through around it.
 */
const useTransparentDocument = () => {
  useEffect(() => {
    const root = document.documentElement;
    const body = document.body;

    const elements: Array<[HTMLElement, Partial<CSSStyleDeclaration>]> = [
      [root, { background: 'transparent' }],
      [
        body,
        {
          // Pinning the body removes the document-level scroll surface
          // entirely, so there's nothing left for WebKit to rubber-band —
          // the window is always resized to fit the card exactly (see
          // useAutoResizeToContent), so there's never real overflow anyway.
          position: 'fixed',
          inset: '0',
          background: 'transparent',
          backgroundImage: 'none',
          overflow: 'hidden',
          minHeight: '0',
        },
      ],
    ];

    const previous = elements.map(([el, styles]) =>
      Object.fromEntries(Object.keys(styles).map((key) => [key, el.style[key as any]]))
    );

    elements.forEach(([el, styles]) => {
      Object.assign(el.style, styles);
    });

    return () => {
      elements.forEach(([el], i) => {
        Object.assign(el.style, previous[i]);
      });
    };
  }, []);
};

/**
 * Keeps the native tray window sized to exactly fit the card's content, so
 * there's never an overflow for the webview to scroll (and thus nothing for
 * macOS's native bounce to rubber-band against).
 */
const useAutoResizeToContent = (ref: React.RefObject<HTMLDivElement | null>) => {
  useEffect(() => {
    if (!isTauri() || !ref.current) return;

    const win = getCurrentWindow();
    const resize = () => {
      if (!ref.current) return;
      // Read offsetHeight (border-box) rather than the observer entry's
      // contentRect (content-box, excludes border/padding) — otherwise the
      // window is sized smaller than the card and its bottom edge is clipped.
      win.setSize(new LogicalSize(TRAY_WIDTH, Math.ceil(ref.current.offsetHeight))).catch(console.error);
    };

    resize();

    const observer = new ResizeObserver(resize);
    observer.observe(ref.current);

    return () => observer.disconnect();
  }, [ref]);
};

export const TrayPopup: React.FC = () => {
  const { drives } = useVault();
  const { isSyncing } = useTelemetry();
  const containerRef = useRef<HTMLDivElement>(null);
  useTransparentDocument();
  useAutoResizeToContent(containerRef);

  return (
    <div
      ref={containerRef}
      className="w-[380px] bg-dot-grid border border-border rounded-xl shadow-[0_25px_60px_rgba(0,0,0,0.22)] text-foreground font-sans pb-4"
      style={{ backgroundSize: '14px 14px' }}
      data-testid="tray-popup-container"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-5 pt-5 pb-3" data-testid="tray-header">
        <div className="font-serif text-[1.45rem] font-bold tracking-tight text-foreground leading-none">
          Phaneros
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => navigateMainWindowToTab('dashboard')}
            aria-label="Launch main window"
            title="Launch main window"
            className="w-8 h-8 rounded-lg bg-card border border-border shadow-card flex items-center justify-center text-foreground hover:border-ring hover:-translate-y-px transition-all"
            data-testid="tray-launch-button"
          >
            ↗
          </button>
          <button
            type="button"
            onClick={() => navigateMainWindowToTab('settings')}
            aria-label="Open settings"
            title="Open settings"
            className="w-8 h-8 rounded-lg bg-card border border-border shadow-card flex items-center justify-center text-foreground hover:border-ring hover:-translate-y-px transition-all"
            data-testid="tray-settings-button"
          >
            ⚙
          </button>
        </div>
      </div>

      {/* Hero Health Card */}
      <div
        className="mx-4 mb-4 mt-1 bg-card border border-border rounded-xl px-5 py-[18px] shadow-card flex items-center justify-between"
        data-testid="tray-hero-health"
      >
        <div className="flex flex-col gap-0.5">
          <div className="font-serif text-[1.35rem] font-bold text-foreground flex items-center gap-2">
            {isSyncing ? 'Syncing 3 Files' : 'Up to Date'}
          </div>
          <div className="text-[0.76rem] text-muted-foreground">
            {isSyncing ? '1.4 MB/s • ~10s remaining' : 'All drives reconciled • 12s ago'}
          </div>
        </div>
        {isSyncing ? (
          <div
            className="w-[42px] h-[42px] rounded-full flex items-center justify-center"
            style={{ background: 'conic-gradient(var(--primary) 68%, var(--muted) 0)' }}
          >
            <div className="w-[30px] h-[30px] rounded-full bg-card flex items-center justify-center text-[0.7rem] font-mono font-bold text-foreground">
              68%
            </div>
          </div>
        ) : (
          <div className="w-[38px] h-[38px] rounded-full bg-emerald-bg border border-emerald-border text-emerald-green flex items-center justify-center text-[1.1rem] font-bold shadow-[0_2px_8px_rgba(5,150,105,0.15)]">
            ✓
          </div>
        )}
      </div>

      {/* Drive Stack */}
      <div className="text-[0.72rem] font-bold uppercase tracking-wider text-muted-foreground px-5 pb-2" data-testid="tray-drive-stack-label">
        Drives
      </div>
      <div className="flex flex-col px-4 gap-3" data-testid="tray-drive-stack">
        {drives.map((drive) => {
          const quotaDisplay = drive.quotaBytes === undefined ? 'Infinite ∞' : `${drive.quotaBytes} B`;
          return (
            <div
              key={drive.id}
              className="group relative bg-card border border-border rounded-xl p-4 shadow-card hover:shadow-card-hover hover:border-ring hover:-translate-y-0.5 transition-all cursor-pointer"
              data-testid={`tray-drive-${drive.id}`}
            >
              <span className="absolute top-2.5 right-3 text-[0.68rem] font-semibold text-primary opacity-0 group-hover:opacity-100 transition-opacity">
                Finder ↗
              </span>
              <div className="flex items-center justify-between">
                <div className="flex flex-col gap-[3px]">
                  <div className="font-bold text-[0.92rem] text-foreground">{drive.name}</div>
                  <div className="text-[0.74rem] text-muted-foreground">{drive.path}</div>
                  <div className="text-[0.72rem] font-semibold text-emerald-green flex items-center gap-1 mt-0.5">
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                    <span>Last synced 2m ago</span>
                  </div>
                </div>
                <div className="flex flex-col items-end gap-1">
                  <div className="w-9 h-9 rounded-full bg-emerald-bg border border-emerald-border text-emerald-green flex items-center justify-center text-[0.85rem] font-bold">
                    ∞
                  </div>
                  <span
                    className="font-mono text-[0.68rem] text-muted-foreground font-semibold"
                    data-testid={`tray-quota-${drive.id}`}
                  >
                    {quotaDisplay}
                  </span>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Recent Activity */}
      <div className="mt-4" data-testid="tray-activity-stream">
        <div className="text-[0.72rem] font-bold uppercase tracking-wider text-muted-foreground px-5 pb-2 flex items-center justify-between">
          <span>Recent File Activity</span>
          <span className="text-[0.65rem] text-primary normal-case tracking-normal font-semibold">Live Stream</span>
        </div>
        <div className="flex flex-col px-4 gap-1.5">
          {mockTrayRecentActivity.map((activity) => (
            <div
              key={activity.id}
              onClick={() => navigateMainWindowToTab('activity')}
              className="bg-card border border-border rounded-lg px-3 py-2 flex items-center justify-between text-[0.78rem] shadow-[0_2px_6px_rgba(0,0,0,0.02)] cursor-pointer"
              data-testid={`tray-activity-item-${activity.id}`}
            >
              <div className="flex items-center gap-2 overflow-hidden">
                <span className="font-mono text-[0.65rem] font-bold uppercase tracking-wide px-1.5 py-0.5 rounded bg-secondary text-secondary-foreground border border-border flex-shrink-0">
                  {activity.ext}
                </span>
                <span className="font-medium text-foreground whitespace-nowrap overflow-hidden text-ellipsis max-w-[200px]">
                  {activity.name}
                </span>
              </div>
              <span className="text-[0.7rem] text-muted-foreground flex-shrink-0">{activity.timestamp}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
