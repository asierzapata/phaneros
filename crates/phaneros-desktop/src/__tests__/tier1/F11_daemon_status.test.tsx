import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
// Deliberately using @testing-library/react directly (not the project's
// `render` helper) — that helper wraps everything in `AllProviders`, which
// already mounts its own `DaemonStatusProvider`; nesting a second one here
// would let the outer provider's own (mocked) polling interfere with the
// call-count assertions below.
import { render, screen } from '@testing-library/react';
import { DaemonStatusProvider, useDaemonStatus } from '@/context/DaemonStatusContext';

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
}));

const pingDaemonMock = vi.fn();
const startDaemonMock = vi.fn();

vi.mock('@/lib/backendBridge', () => ({
  pingDaemon: (...args: unknown[]) => pingDaemonMock(...args),
  startDaemon: (...args: unknown[]) => startDaemonMock(...args),
}));

const Probe: React.FC = () => {
  const { connectionState, configured, isStartingDaemon, startDaemon } = useDaemonStatus();
  return (
    <div>
      <div data-testid="connection-state">{connectionState}</div>
      <div data-testid="configured">{String(configured)}</div>
      <div data-testid="is-starting">{String(isStartingDaemon)}</div>
      <button data-testid="start-button" onClick={() => startDaemon()}>
        Start
      </button>
    </div>
  );
};

describe('F11_DAEMON_STATUS: DaemonStatusContext', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    pingDaemonMock.mockReset();
    startDaemonMock.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('F11-T1-01: starts in the "checking" state before the first ping resolves', () => {
    pingDaemonMock.mockReturnValue(new Promise(() => {})); // never resolves
    render(
      <DaemonStatusProvider>
        <Probe />
      </DaemonStatusProvider>
    );

    expect(screen.getByTestId('connection-state')).toHaveTextContent('checking');
  });

  it('F11-T1-02: transitions to "reachable" and picks up `configured` on a successful ping', async () => {
    pingDaemonMock.mockResolvedValue({ version: '0.1.0', configured: true });
    render(
      <DaemonStatusProvider>
        <Probe />
      </DaemonStatusProvider>
    );

    await vi.waitFor(() => {
      expect(screen.getByTestId('connection-state')).toHaveTextContent('reachable');
    });
    expect(screen.getByTestId('configured')).toHaveTextContent('true');
  });

  it('F11-T1-03: transitions to "unreachable" when the ping rejects', async () => {
    pingDaemonMock.mockRejectedValue(new Error('Is phanerosd running?'));
    render(
      <DaemonStatusProvider>
        <Probe />
      </DaemonStatusProvider>
    );

    await vi.waitFor(() => {
      expect(screen.getByTestId('connection-state')).toHaveTextContent('unreachable');
    });
    expect(screen.getByTestId('configured')).toHaveTextContent('null');
  });

  it('F11-T1-04: polls repeatedly on an interval', async () => {
    pingDaemonMock.mockResolvedValue({ version: '0.1.0', configured: true });
    render(
      <DaemonStatusProvider>
        <Probe />
      </DaemonStatusProvider>
    );

    await vi.waitFor(() => expect(pingDaemonMock).toHaveBeenCalledTimes(1));

    await vi.advanceTimersByTimeAsync(3000);
    expect(pingDaemonMock).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(3000);
    expect(pingDaemonMock).toHaveBeenCalledTimes(3);
  });

  it('F11-T1-05: startDaemon() calls the bridge and re-checks connectivity afterwards', async () => {
    pingDaemonMock
      .mockResolvedValueOnce({ version: '0.1.0', configured: false }) // initial mount ping
      .mockResolvedValue({ version: '0.1.0', configured: true }); // post-start recheck
    startDaemonMock.mockResolvedValue(undefined);

    render(
      <DaemonStatusProvider>
        <Probe />
      </DaemonStatusProvider>
    );

    await vi.waitFor(() => expect(pingDaemonMock).toHaveBeenCalledTimes(1));

    screen.getByTestId('start-button').click();

    await vi.waitFor(() => expect(startDaemonMock).toHaveBeenCalledTimes(1));

    await vi.advanceTimersByTimeAsync(1000);

    await vi.waitFor(() => expect(pingDaemonMock).toHaveBeenCalledTimes(2));
    await vi.waitFor(() => {
      expect(screen.getByTestId('configured')).toHaveTextContent('true');
    });
  });
});
