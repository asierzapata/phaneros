import { describe, it, expect } from 'vitest';
import { render, screen } from '../helpers/render';
import userEvent from '@testing-library/user-event';
import { CodeDiff, BinaryMetadataDiff } from '@/types';
import React, { useState } from 'react';

const ConflictResolverBoundaryComponent: React.FC<{
  textConflicts?: CodeDiff[];
  binaryConflicts?: BinaryMetadataDiff[];
}> = ({ textConflicts = [], binaryConflicts = [] }) => {
  const [activeText, setActiveText] = useState<CodeDiff[]>(textConflicts);
  const [activeBinary, setActiveBinary] = useState<BinaryMetadataDiff[]>(binaryConflicts);
  const [selectedAction, setSelectedAction] = useState<Record<string, 'Keep Local' | 'Keep Store'>>({});

  const totalConflicts = activeText.length + activeBinary.length;

  const resolveTextConflict = (filename: string) => {
    setActiveText((prev) => prev.filter((item) => item.filename !== filename));
  };

  const resolveBinaryConflict = (filename: string) => {
    setActiveBinary((prev) => prev.filter((item) => item.filename !== filename));
  };

  return (
    <div data-testid="conflicts-root" className="p-6">
      <h2>Conflict Resolution Workspace</h2>
      {totalConflicts === 0 ? (
        <div data-testid="all-resolved-banner">All conflicts resolved. Your workspace is up to date.</div>
      ) : (
        <div>
          {/* Text Code Diffs */}
          {activeText.map((diff) => (
            <div key={diff.filename} data-testid={`text-diff-${diff.filename}`} className="mb-6 border p-4">
              <h3>{diff.filename}</h3>
              {diff.linesAdded === 0 && diff.linesRemoved === 0 ? (
                <div data-testid={`no-diff-msg-${diff.filename}`}>No differences found in file contents.</div>
              ) : (
                <div>
                  {diff.chunks.map((chunk, cIdx) => (
                    <div key={cIdx} data-testid={`chunk-${cIdx}`}>
                      {chunk.lines.map((line, lIdx) => (
                        <div
                          key={lIdx}
                          data-testid={`diff-line-${cIdx}-${lIdx}`}
                          className={
                            line.type === 'add'
                              ? 'bg-emerald-500/20 diffs-word-added'
                              : line.type === 'delete'
                              ? 'bg-red-500/20 diffs-word-removed'
                              : ''
                          }
                        >
                          {line.text}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              )}
              <button
                data-testid={`resolve-text-btn-${diff.filename}`}
                onClick={() => resolveTextConflict(diff.filename)}
                className="mt-2 px-3 py-1 bg-emerald-600 text-white rounded"
              >
                Resolve Conflict
              </button>
            </div>
          ))}

          {/* Binary Metadata Matrix */}
          {activeBinary.map((binary) => {
            const currentAction = selectedAction[binary.filename] || binary.recommendedAction;

            return (
              <div key={binary.filename} data-testid={`binary-diff-${binary.filename}`} className="border p-4">
                <h3>{binary.filename} (Binary)</h3>
                <div data-testid={`recommended-action-${binary.filename}`}>Recommended: {binary.recommendedAction}</div>
                <div className="grid grid-cols-2 gap-4 my-2">
                  <div>
                    <h4>Local Copy</h4>
                    <p data-testid={`binary-local-size-${binary.filename}`}>Size: {binary.local.size}</p>
                    <p data-testid={`binary-local-hash-${binary.filename}`}>Hash: {binary.local.hash || 'N/A'}</p>
                  </div>
                  <div>
                    <h4>Store Copy</h4>
                    <p data-testid={`binary-store-size-${binary.filename}`}>Size: {binary.store.size}</p>
                    <p data-testid={`binary-store-hash-${binary.filename}`}>Hash: {binary.store.hash || 'N/A'}</p>
                  </div>
                </div>

                <div className="flex space-x-2">
                  <button
                    data-testid={`btn-select-local-${binary.filename}`}
                    onClick={() => setSelectedAction((prev) => ({ ...prev, [binary.filename]: 'Keep Local' }))}
                    className={`px-2 py-1 border ${currentAction === 'Keep Local' ? 'bg-primary text-white' : ''}`}
                  >
                    Keep Local
                  </button>
                  <button
                    data-testid={`btn-select-store-${binary.filename}`}
                    onClick={() => setSelectedAction((prev) => ({ ...prev, [binary.filename]: 'Keep Store' }))}
                    className={`px-2 py-1 border ${currentAction === 'Keep Store' ? 'bg-primary text-white' : ''}`}
                  >
                    Keep Store
                  </button>
                  <button
                    data-testid={`resolve-binary-btn-${binary.filename}`}
                    onClick={() => resolveBinaryConflict(binary.filename)}
                    className="px-3 py-1 bg-emerald-600 text-white rounded"
                  >
                    Apply Choice ({currentAction})
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

describe('F7_DIFF: Diffs.com Conflict Resolver Boundary Tests', () => {
  it('F7-T2-01: should render "No differences found" message for 0-change identical text file diffs', () => {
    const zeroDiff: CodeDiff = {
      filename: 'identical.txt',
      path: '~/Documents/identical.txt',
      linesAdded: 0,
      linesRemoved: 0,
      chunks: [],
    };

    render(<ConflictResolverBoundaryComponent textConflicts={[zeroDiff]} />);

    expect(screen.getByTestId('no-diff-msg-identical.txt')).toHaveTextContent('No differences found');
  });

  it('F7-T2-02: should handle large file diff chunks with 100+ lines without breaking DOM rendering', () => {
    const largeLines = Array.from({ length: 120 }, (_, i) => ({
      type: (i % 2 === 0 ? 'add' : 'delete') as 'add' | 'delete',
      text: `Line content #${i + 1}`,
    }));

    const largeDiff: CodeDiff = {
      filename: 'LargeFile.rs',
      path: '~/Developer/LargeFile.rs',
      linesAdded: 60,
      linesRemoved: 60,
      chunks: [{ oldStart: 1, newStart: 1, lines: largeLines }],
    };

    render(<ConflictResolverBoundaryComponent textConflicts={[largeDiff]} />);

    expect(screen.getByTestId('diff-line-0-0')).toBeInTheDocument();
    expect(screen.getByTestId('diff-line-0-119')).toBeInTheDocument();
  });

  it('F7-T2-03: should display "N/A" fallback when binary metadata local/store hash is empty or missing', () => {
    const missingHashBinary: BinaryMetadataDiff = {
      filename: 'corrupted.sqlite',
      path: '~/Documents/corrupted.sqlite',
      isBinary: true,
      local: { size: '4.2 MB', modified: '2026-08-04', hash: '' },
      store: { size: '4.2 MB', modified: '2026-08-04', hash: '' },
      recommendedAction: 'Keep Local',
    };

    render(<ConflictResolverBoundaryComponent binaryConflicts={[missingHashBinary]} />);

    expect(screen.getByTestId('binary-local-hash-corrupted.sqlite')).toHaveTextContent('Hash: N/A');
    expect(screen.getByTestId('binary-store-hash-corrupted.sqlite')).toHaveTextContent('Hash: N/A');
  });

  it('F7-T2-04: should display all resolved banner when all conflicts are cleared from queue', async () => {
    const user = userEvent.setup();
    const singleConflict: CodeDiff = {
      filename: 'single.md',
      path: '~/Documents/single.md',
      linesAdded: 1,
      linesRemoved: 0,
      chunks: [{ oldStart: 1, newStart: 1, lines: [{ type: 'add', text: '+ New line' }] }],
    };

    render(<ConflictResolverBoundaryComponent textConflicts={[singleConflict]} />);

    const resolveBtn = screen.getByTestId('resolve-text-btn-single.md');
    await user.click(resolveBtn);

    expect(screen.getByTestId('all-resolved-banner')).toHaveTextContent('All conflicts resolved');
  });

  it('F7-T2-05: should allow overriding recommended action from Keep Local to Keep Store', async () => {
    const user = userEvent.setup();
    const binaryDiff: BinaryMetadataDiff = {
      filename: 'data.bin',
      path: '~/Documents/data.bin',
      isBinary: true,
      local: { size: '10 MB', modified: '12:00', hash: 'abc1' },
      store: { size: '12 MB', modified: '12:05', hash: 'def2' },
      recommendedAction: 'Keep Local',
    };

    render(<ConflictResolverBoundaryComponent binaryConflicts={[binaryDiff]} />);

    const storeBtn = screen.getByTestId('btn-select-store-data.bin');
    await user.click(storeBtn);

    const resolveBtn = screen.getByTestId('resolve-binary-btn-data.bin');
    expect(resolveBtn).toHaveTextContent('Apply Choice (Keep Store)');
  });
});
