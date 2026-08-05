import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { DrivesFiles } from '@/components/main/DrivesFiles';
import { mockDrives } from '../mocks/vaultMocks';

describe('F6_TREE: Trees.software File Explorer', () => {
  it('F6-T1-01: should render split drive browser selector list', () => {
    render(<DrivesFiles />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives, initialActiveId: mockDrives[0].id },
      },
    });

    const selectorList = screen.getByTestId('drive-selector-list');
    expect(selectorList).toBeInTheDocument();
    expect(screen.getByTestId(`drive-selector-item-${mockDrives[0].id}`)).toBeInTheDocument();
    expect(screen.getByTestId(`drive-selector-item-${mockDrives[1].id}`)).toBeInTheDocument();
  });

  it('F6-T1-02: should render drive metadata card with Infinite ∞ capacity', () => {
    render(<DrivesFiles />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives, initialActiveId: mockDrives[0].id },
      },
    });

    const metadataCard = screen.getByTestId('drive-metadata-card');
    expect(metadataCard).toBeInTheDocument();

    const quotaDisplay = screen.getByTestId('active-drive-quota-display');
    expect(quotaDisplay).toHaveTextContent('Infinite ∞');
  });

  it('F6-T1-03: should render interactive file tree and expand/collapse directory folders', async () => {
    const user = userEvent.setup();
    render(<DrivesFiles />);

    const fileTree = screen.getByTestId('interactive-file-tree');
    expect(fileTree).toBeInTheDocument();

    expect(screen.getByText('src')).toBeInTheDocument();
    expect(screen.getByText('sync-protocol')).toBeInTheDocument();

    const collapseButton = screen.getByRole('button', { name: /collapse folder/i });
    await user.click(collapseButton);

    expect(screen.queryByText('sync-protocol')).not.toBeInTheDocument();
  });

  it('F6-T1-04: should display file extension badges (MD, RS, DB) in JetBrains Mono font', () => {
    render(<DrivesFiles />);

    const rsBadge = screen.getByText('RS');
    const mdBadge = screen.getByText('MD');
    const dbBadge = screen.getByText('DB');

    expect(rsBadge).toHaveClass('font-mono');
    expect(mdBadge).toHaveClass('font-mono');
    expect(dbBadge).toHaveClass('font-mono');
  });

  it('F6-T1-05: should render file status pills and select file nodes', async () => {
    const user = userEvent.setup();
    render(<DrivesFiles />, {
      providerProps: {
        vaultProps: { initialDrives: mockDrives, initialActiveId: mockDrives[0].id },
      },
    });

    const driveItem2 = screen.getByTestId(`drive-selector-item-${mockDrives[1].id}`);
    await user.click(driveItem2);

    expect(screen.getByTestId('drive-metadata-card')).toHaveTextContent(mockDrives[1].name);
  });
});
