import { describe, it, expect } from 'vitest';
import { render, screen, within } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { Conflicts } from '@/components/main/Conflicts';
import { mockBinaryConflict } from '../mocks/diffMocks';

describe('F7_DIFF: Diffs.com Conflict Resolver', () => {
  it('F7-T1-01: should render side-by-side text diffing workspace for text/code files', () => {
    render(<Conflicts />);
    const diffWorkspace = screen.getByTestId('side-by-side-diff');
    expect(diffWorkspace).toBeInTheDocument();
    expect(screen.getByText('Local Version')).toBeInTheDocument();
    expect(screen.getByText('Server Version')).toBeInTheDocument();
  });

  it('F7-T1-02: should render word-level diff highlights with .diffs-word-added and .diffs-word-removed', () => {
    render(<Conflicts />);
    const diffWorkspace = within(screen.getByTestId('side-by-side-diff'));
    const addedWord = diffWorkspace.getByText('End-to-end');
    expect(addedWord).toHaveClass('diffs-word-added');

    const removedWord = diffWorkspace.getByText('Local');
    expect(removedWord).toHaveClass('diffs-word-removed');
  });

  it('F7-T1-03: should render Keep Local and Keep Server toolbar action buttons', async () => {
    const user = userEvent.setup();
    render(<Conflicts />);

    const keepLocalBtn = screen.getByRole('button', { name: /keep local/i });
    const keepServerBtn = screen.getByRole('button', { name: /keep server/i });

    expect(keepLocalBtn).toBeInTheDocument();
    expect(keepServerBtn).toBeInTheDocument();

    await user.click(keepLocalBtn);
    expect(screen.getByTestId('resolution-banner')).toHaveTextContent(/Resolved: Kept Local Copy/i);
  });

  it('F7-T1-04: should render binary metadata comparison matrix table for opaque binary files (.sqlite)', async () => {
    const user = userEvent.setup();
    render(<Conflicts />);

    const binarySwitchBtn = screen.getByTestId('switch-binary-conflict');
    await user.click(binarySwitchBtn);

    const matrix = screen.getByTestId('binary-matrix');
    expect(matrix).toBeInTheDocument();
    expect(screen.getByText('Binary File Comparison Matrix')).toBeInTheDocument();
  });

  it('F7-T1-05: should display binary comparison details (size, modified timestamp, SHA-256 hash, recommended action)', async () => {
    const user = userEvent.setup();
    render(<Conflicts />);

    await user.click(screen.getByTestId('switch-binary-conflict'));
    const binaryMatrix = within(screen.getByTestId('binary-matrix'));

    expect(binaryMatrix.getByText(mockBinaryConflict.local.size)).toBeInTheDocument();
    expect(binaryMatrix.getByText(mockBinaryConflict.store.size)).toBeInTheDocument();
    expect(binaryMatrix.getByText(mockBinaryConflict.local.modified)).toBeInTheDocument();
    expect(binaryMatrix.getByText(mockBinaryConflict.local.hash)).toBeInTheDocument();
    expect(binaryMatrix.getByText(mockBinaryConflict.recommendedAction)).toBeInTheDocument();
  });
});
